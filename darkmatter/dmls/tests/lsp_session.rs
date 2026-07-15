//! Level-1 integration tests: full LSP conversations over an in-memory
//! connection pair (`lsp_server::Connection::memory()`), the fixture shape
//! ported from `iwes/tests/fixture.rs` (Apache-2.0, IWE project).
//!
//! These sessions are in-memory (no real terminal, PTY, or network resource),
//! so they run in the standard `just test` gate with no terminal harness.

use std::sync::mpsc;
use std::time::Duration;

use dmls::overlay::expressions;
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
    /// Server → client requests buffered while awaiting a response (the server
    /// pushes `workspace/semanticTokens/refresh`, progress-create, and watcher
    /// registrations fire-and-forget).
    server_requests: Vec<Request>,
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
            server_requests: Vec::new(),
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
                // registration, semantic-tokens refresh) are fire-and-forget;
                // buffer them so tests can observe what the server pushed.
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for response: {other:?}"),
            }
        }
    }

    /// Waits for the latest `publishDiagnostics` for `uri`, returning its
    /// `diagnostics` array. Drains buffered notifications first.
    fn wait_for_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        self.wait_for_diagnostics_params(uri)["diagnostics"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    fn wait_for_diagnostics_params(&mut self, uri: &str) -> Value {
        loop {
            if let Some(params) = self.take_buffered_diagnostics(uri) {
                return params;
            }
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("diagnostics before timeout");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for diagnostics: {other:?}"),
            }
        }
    }

    fn take_buffered_diagnostics(&mut self, uri: &str) -> Option<Value> {
        let position = self.notifications.iter().rposition(|notification| {
            notification.method == "textDocument/publishDiagnostics"
                && notification.params["uri"] == json!(uri)
        })?;
        let notification = self.notifications.remove(position);
        Some(notification.params)
    }

    /// Drains messages until the startup-index `workDoneProgress` `end` arrives,
    /// guaranteeing the background disk walk has finished. Only usable when the
    /// client advertised `window.workDoneProgress`.
    fn wait_for_startup_complete(&mut self) {
        let is_end = |notification: &Notification| {
            notification.method == "$/progress"
                && notification.params["token"] == json!("dmls/startup-index")
                && notification.params["value"]["kind"] == json!("end")
        };
        loop {
            if let Some(position) = self.notifications.iter().position(is_end) {
                self.notifications.remove(position);
                return;
            }
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("startup progress before timeout");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for startup: {other:?}"),
            }
        }
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

    fn flush_server(&mut self) {
        let response = self.request("workspace/symbol", json!({ "query": "" }));
        assert!(response.error.is_none(), "server flush request failed: {:?}", response.error);
    }

    /// Pumps messages until a buffered server-initiated request with `method`
    /// is seen, removing and returning `true`; returns `false` if none arrives
    /// within a short window (the in-memory pair delivers immediately, so a
    /// miss reliably means the server never sent it).
    fn wait_for_server_request(&mut self, method: &str) -> bool {
        self.take_server_request(method).is_some()
    }

    /// Like [`wait_for_server_request`], but returns the matched `Request` so a
    /// caller can inspect its `id` (e.g. proving two refreshes carry distinct
    /// request ids); `None` on the same short-window miss.
    fn take_server_request(&mut self, method: &str) -> Option<Request> {
        loop {
            if let Some(position) =
                self.server_requests.iter().position(|request| request.method == method)
            {
                return Some(self.server_requests.remove(position));
            }
            match self.client.receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(Message::Request(request)) => self.server_requests.push(request),
                Ok(Message::Notification(notification)) => self.notifications.push(notification),
                Ok(Message::Response(_)) => {}
                Err(_) => return None,
            }
        }
    }

    /// Sends a client → server response, e.g. answering a server-initiated
    /// `workspace/semanticTokens/refresh` so it routes through the loop's
    /// `Message::Response` arm and into the `RefreshLedger`.
    fn respond(&self, response: Response) {
        self.client
            .sender
            .send(Message::Response(response))
            .expect("send response");
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

fn watched_initialize_params(root: &std::path::Path) -> Value {
    let mut params = neovim_like_initialize_params(root);
    params["clientInfo"] = json!({ "name": "Watched Client", "version": "1.0" });
    params["capabilities"]["workspace"]["didChangeWatchedFiles"] =
        json!({ "dynamicRegistration": true });
    params
}

#[test]
fn initialize_open_change_shutdown() {
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

    // A genuinely unimplemented method still answers MethodNotFound. Range
    // formatting is explicitly post-v1 (only whole-document formatting ships).
    let response = fixture.request(
        "textDocument/rangeFormatting",
        json!({
            "textDocument": { "uri": doc_uri.as_str() },
            "range": { "start": { "line": 0, "character": 0 }, "end": { "line": 0, "character": 0 } },
            "options": { "tabSize": 2, "insertSpaces": true }
        }),
    );
    assert_eq!(response.error.expect("rangeFormatting unimplemented").code, -32601);

    fixture.shutdown();
}

#[test]
fn default_negotiation_is_utf16() {
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

/// Requests `textDocument/hover` at `(line, character)` and returns the rendered
/// Markdown body (empty string on a null hover). Collapses the request +
/// `contents.value` extraction that hover assertions would otherwise repeat.
fn hover_markup(fixture: &mut ClientFixture, uri: &str, line: u32, character: u32) -> String {
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .expect("hover result");
    hover["contents"]["value"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn layer0_provider_round_trips() {
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
fn broken_link_diagnostic_updates_on_edit() {
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
fn server_rescan_fallback_tracks_unopened_files_on_save() {
    // Spec acceptance criterion 2 for watcher-less clients: a ServerRescan
    // client (Neovim-on-Linux shape — no reliable file watcher) must still see
    // an unopened file's create/delete. Its only trigger is a save, so a
    // `didSave` drives a workspace rescan that reconciles the graph.
    let workspace = tempfile::tempdir().unwrap();
    let a_path = workspace.path().join("a.md");
    let b_path = workspace.path().join("b.md");
    let a_text = "# A\n\n[to b](b.md)\n";
    // a.md links to b.md, which does not exist yet.
    std::fs::write(&a_path, a_text).unwrap();

    // Advertise workDoneProgress so the test can wait for the startup disk walk
    // to finish; otherwise the background scan could adopt b.md itself and mask
    // the save-driven rescan under test. Neovim's keyed defaults still select
    // `ServerRescan` regardless of the progress capability.
    let mut init = neovim_like_initialize_params(workspace.path());
    init["capabilities"]["window"] = json!({ "workDoneProgress": true });

    let mut fixture = ClientFixture::start();
    fixture.initialize(init);
    fixture.wait_for_startup_complete();

    let a_uri = url::Url::from_file_path(&a_path).unwrap();
    open(&fixture, a_uri.as_str(), a_text);

    // The unopened, absent b.md makes the link broken.
    let diagnostics = fixture.wait_for_diagnostics(a_uri.as_str());
    assert!(
        diagnostics.iter().any(|d| d["code"] == json!("dm.links.broken_path")),
        "expected a broken-link diagnostic while b.md is absent: {diagnostics:?}"
    );

    // Create b.md on disk (never opened) and save a.md: the rescan adopts it and
    // the link resolves.
    std::fs::write(&b_path, "# B\n").unwrap();
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": a_uri.as_str() } }),
    );
    let diagnostics = fixture.wait_for_diagnostics(a_uri.as_str());
    assert!(
        diagnostics.is_empty(),
        "creating b.md on disk should clear the broken link after a save: {diagnostics:?}"
    );

    // Delete b.md and save again: the rescan drops it and the link breaks anew.
    std::fs::remove_file(&b_path).unwrap();
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": a_uri.as_str() } }),
    );
    let diagnostics = fixture.wait_for_diagnostics(a_uri.as_str());
    assert!(
        diagnostics.iter().any(|d| d["code"] == json!("dm.links.broken_path")),
        "deleting b.md on disk should re-break the link after a save: {diagnostics:?}"
    );

    fixture.shutdown();
}

#[test]
fn trigger_payload_failure_retains_effective_schema_and_diagnoses_envelope() {
    let workspace = tempfile::tempdir().unwrap();
    let schemas = workspace.path().join("schemas");
    std::fs::create_dir(&schemas).unwrap();
    std::fs::write(workspace.path().join(".dmls.toml"), "[schema]\nstrict = true\n").unwrap();
    let envelope_path = schemas.join("prompt.trigger.yaml");
    let payload_path = schemas.join("prompt.yaml");
    let document_path = workspace.path().join("prompt.md");
    let envelope = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
    let payload = "$schema:\n  model: string(required)\n";
    let document = "---\nprompt: hello\n---\n\nBody\n";
    std::fs::write(&envelope_path, envelope).unwrap();
    std::fs::write(&payload_path, payload).unwrap();
    std::fs::write(&document_path, document).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let envelope_uri = url::Url::from_file_path(&envelope_path).unwrap();
    let document_uri = url::Url::from_file_path(&document_path).unwrap();
    open(&fixture, envelope_uri.as_str(), envelope);
    let initial_envelope = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(initial_envelope.is_empty(), "valid envelope diagnostics: {initial_envelope:?}");
    open(&fixture, document_uri.as_str(), document);

    let initial = fixture.wait_for_diagnostics(document_uri.as_str());
    assert!(
        initial.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.missing_required")
                && diagnostic["message"].as_str().is_some_and(|message| message.contains("model"))
        }),
        "the trigger must initially contribute its required property: {initial:?}"
    );
    let refreshed_envelope = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(
        refreshed_envelope.is_empty(),
        "valid envelope diagnostics after consumer open: {refreshed_envelope:?}"
    );

    std::fs::remove_file(&payload_path).unwrap();
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": envelope_uri.as_str() } }),
    );

    let envelope_diagnostics = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(
        envelope_diagnostics.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.prepare")
                && diagnostic["source"] == json!("darkmatter.schema")
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.contains("prompt.trigger.yaml"))
        }),
        "the failed scan must publish a schema-load diagnostic on the envelope: \
         {envelope_diagnostics:?}"
    );

    let retained = fixture.wait_for_diagnostics(document_uri.as_str());
    assert!(
        retained.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.missing_required")
                && diagnostic["message"].as_str().is_some_and(|message| message.contains("model"))
        }),
        "the consumer must retain the prior effective schema after payload removal: {retained:?}"
    );

    fixture.shutdown();
}

fn closed_trigger_envelope_diagnostic_round_trip(client_watched: bool) {
    let workspace = tempfile::tempdir().unwrap();
    let schemas = workspace.path().join("schemas");
    std::fs::create_dir(&schemas).unwrap();
    std::fs::write(workspace.path().join(".dmls.toml"), "[schema]\nstrict = true\n").unwrap();
    let envelope_path = schemas.join("prompt.trigger.yaml");
    let payload_path = schemas.join("prompt.yaml");
    let document_path = workspace.path().join("prompt.md");
    let envelope = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
    let payload = "$schema:\n  model: string(required)\n";
    let document = "---\nprompt: hello\n---\n\nBody\n";
    std::fs::write(&envelope_path, envelope).unwrap();
    std::fs::write(&payload_path, payload).unwrap();
    std::fs::write(&document_path, document).unwrap();

    let mut fixture = ClientFixture::start();
    let init = if client_watched {
        watched_initialize_params(workspace.path())
    } else {
        neovim_like_initialize_params(workspace.path())
    };
    fixture.initialize(init);
    let envelope_uri = url::Url::from_file_path(&envelope_path).unwrap();
    let payload_uri = url::Url::from_file_path(&payload_path).unwrap();
    let document_uri = url::Url::from_file_path(&document_path).unwrap();
    open(&fixture, document_uri.as_str(), document);
    let initial = fixture.wait_for_diagnostics(document_uri.as_str());
    assert!(
        initial.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.missing_required")
                && diagnostic["message"].as_str().is_some_and(|message| message.contains("model"))
        }),
        "the trigger must initially contribute its required property: {initial:?}"
    );

    std::fs::remove_file(&payload_path).unwrap();
    if client_watched {
        fixture.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [{ "uri": payload_uri.as_str(), "type": 3 }] }),
        );
    } else {
        fixture.notify(
            "textDocument/didSave",
            json!({ "textDocument": { "uri": document_uri.as_str() } }),
        );
    }

    let failed = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(
        failed.iter().any(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.prepare")
                && diagnostic["source"] == json!("darkmatter.schema")
        }),
        "an unopened envelope must receive the failed registry diagnostic: {failed:?}"
    );
    let retained = fixture.wait_for_diagnostics(document_uri.as_str());
    assert!(
        retained.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.missing_required")),
        "the consumer must retain its last-good trigger schema: {retained:?}"
    );

    std::fs::write(&payload_path, payload).unwrap();
    if client_watched {
        fixture.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [{ "uri": payload_uri.as_str(), "type": 1 }] }),
        );
    } else {
        fixture.notify(
            "textDocument/didSave",
            json!({ "textDocument": { "uri": document_uri.as_str() } }),
        );
    }
    let repaired = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(repaired.is_empty(), "repair must clear the unopened envelope: {repaired:?}");

    fixture.shutdown();
}

#[test]
fn server_rescan_publishes_and_clears_unopened_trigger_envelope_diagnostic() {
    closed_trigger_envelope_diagnostic_round_trip(false);
}

#[test]
fn client_watcher_publishes_and_clears_unopened_trigger_envelope_diagnostic() {
    closed_trigger_envelope_diagnostic_round_trip(true);
}

#[test]
fn repairing_trigger_payload_clears_open_envelope_immediately() {
    let workspace = tempfile::tempdir().unwrap();
    let schemas = workspace.path().join("schemas");
    std::fs::create_dir(&schemas).unwrap();
    let envelope_path = schemas.join("prompt.trigger.yaml");
    let payload_path = schemas.join("prompt.yaml");
    let document_path = workspace.path().join("prompt.md");
    let envelope = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
    let document = "---\nprompt: hello\n---\n\nBody\n";
    std::fs::write(&envelope_path, envelope).unwrap();
    std::fs::write(&document_path, document).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let envelope_uri = url::Url::from_file_path(&envelope_path).unwrap();
    let document_uri = url::Url::from_file_path(&document_path).unwrap();
    open(&fixture, envelope_uri.as_str(), envelope);
    open(&fixture, document_uri.as_str(), document);
    fixture.flush_server();
    let failed = fixture.wait_for_diagnostics(envelope_uri.as_str());
    assert!(
        failed.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare")),
        "the open envelope must receive its trigger-load diagnostic: {failed:?}"
    );
    let _ = fixture.wait_for_diagnostics(document_uri.as_str());

    std::fs::write(&payload_path, "$schema:\n  model: string(required)\n").unwrap();
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": document_uri.as_str() } }),
    );
    fixture.flush_server();

    let repaired = fixture.wait_for_diagnostics_params(envelope_uri.as_str());
    assert_eq!(repaired["version"], json!(1), "open-envelope diagnostics must be versioned");
    assert_eq!(repaired["diagnostics"], json!([]), "repair must clear the open envelope");

    fixture.shutdown();
}

fn open_trigger_failure_transfer_round_trip(envelope_first: bool) {
    let workspace = tempfile::tempdir().unwrap();
    let schemas = workspace.path().join("schemas");
    std::fs::create_dir(&schemas).unwrap();
    let envelope_a_path = schemas.join("a.trigger.yaml");
    let envelope_b_path = schemas.join("b.trigger.yaml");
    let payload_a_path = schemas.join("a.yaml");
    let payload_b_path = schemas.join("b.yaml");
    let document_path = workspace.path().join("consumer.md");
    let envelope_a = "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: a.yaml\n";
    let envelope_b = "kind: trigger-schema\nmatch:\n  other: string(required)\n$schema: b.yaml\n";
    let document = "---\nprompt: hello\n---\n\nBody\n";
    std::fs::write(&envelope_a_path, envelope_a).unwrap();
    std::fs::write(&envelope_b_path, envelope_b).unwrap();
    std::fs::write(&payload_b_path, "$schema:\n  beta: string\n").unwrap();
    std::fs::write(&document_path, document).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let envelope_a_uri = url::Url::from_file_path(&envelope_a_path).unwrap();
    let envelope_b_uri = url::Url::from_file_path(&envelope_b_path).unwrap();
    let document_uri = url::Url::from_file_path(&document_path).unwrap();
    if envelope_first {
        open(&fixture, envelope_a_uri.as_str(), envelope_a);
        open(&fixture, document_uri.as_str(), document);
    } else {
        open(&fixture, document_uri.as_str(), document);
        open(&fixture, envelope_a_uri.as_str(), envelope_a);
    }
    fixture.flush_server();
    let _ = fixture.wait_for_diagnostics(document_uri.as_str());
    let failed_a = fixture.wait_for_diagnostics(envelope_a_uri.as_str());
    assert!(
        failed_a.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare")),
        "envelope A must initially own the trigger-load diagnostic: {failed_a:?}"
    );

    std::fs::write(&payload_a_path, "$schema:\n  alpha: string\n").unwrap();
    std::fs::remove_file(&payload_b_path).unwrap();
    fixture.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": document_uri.as_str() } }),
    );
    fixture.flush_server();

    let repaired_a = fixture.wait_for_diagnostics_params(envelope_a_uri.as_str());
    assert_eq!(repaired_a["version"], json!(1), "open envelope A must be versioned");
    assert_eq!(repaired_a["diagnostics"], json!([]), "envelope A must clear immediately");
    let failed_b = fixture.wait_for_diagnostics_params(envelope_b_uri.as_str());
    assert!(failed_b["version"].is_null(), "closed envelope B must remain file-level");
    assert!(
        failed_b["diagnostics"].as_array().is_some_and(|diagnostics| {
            diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare"))
        }),
        "envelope B must receive transferred trigger-load ownership immediately: {failed_b:?}"
    );

    fixture.shutdown();
}

#[test]
fn trigger_failure_transfer_republishes_open_envelope_for_either_open_order() {
    open_trigger_failure_transfer_round_trip(true);
    open_trigger_failure_transfer_round_trip(false);
}

#[test]
fn wiki_link_navigation_diagnostics_and_completion() {
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

#[test]
fn config_reload_reindexes_wiki_roots() {
    // A `workspace/didChangeConfiguration` that narrows `wiki.wiki_root` must
    // rebuild the wiki resolution universe and re-publish diagnostics without a
    // server restart: a link that resolves under the default roots becomes
    // unresolved once the root is narrowed.
    let workspace = tempfile::tempdir().unwrap();
    let notes = workspace.path().join("notes");
    std::fs::create_dir_all(&notes).unwrap();
    std::fs::write(notes.join("Target.md"), "# Target\n").unwrap();
    // A root-relative link whose two segments only match Target's canonical path
    // while the wiki root is the workspace folder (`notes/Target`).
    let source = "# Source\n\n[[/notes/Target]]\n";
    std::fs::write(notes.join("Source.md"), source).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let source_uri = url::Url::from_file_path(notes.join("Source.md")).unwrap();
    let target_uri = url::Url::from_file_path(notes.join("Target.md")).unwrap();
    open(&fixture, target_uri.as_str(), "# Target\n");
    open(&fixture, source_uri.as_str(), source);

    // Default wiki roots (the workspace folder) resolve the root-relative link,
    // so no unresolved-wiki diagnostic is published.
    let diagnostics = fixture.wait_for_diagnostics(source_uri.as_str());
    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"] == json!("wiki.unresolved-target")),
        "link should resolve under default wiki roots: {diagnostics:?}"
    );

    // Narrow `wiki.wiki_root` to `notes`: Target's canonical path shortens to
    // `Target`, so the two-segment root-relative link no longer matches.
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "wiki": { "wiki_root": "notes" } } } }),
    );

    // The reload rebuilds the graph and re-publishes: the link is now unresolved.
    let diagnostics = fixture.wait_for_diagnostics(source_uri.as_str());
    assert!(
        diagnostics
            .iter()
            .any(|d| d["code"] == json!("wiki.unresolved-target")
                && d["source"] == json!("darkmatter.wiki")),
        "narrowing wiki_root without restart should break the link: {diagnostics:?}"
    );

    fixture.shutdown();
}

/// A frontmatter document with an inline `$schema` declaring a required
/// `title` and an enum `status`; `title` is intentionally missing.
const SCHEMA_DOC: &str = "---\n$schema:\n  title: string(required)\n  status: enum(draft, published)\nstatus: draft\n---\n\n# Doc\n";

#[test]
fn frontmatter_schema_intelligence() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), SCHEMA_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), SCHEMA_DOC);

    // `required` is a compose-time contract (the value is injected via CLI /
    // seed / interactive prompt), so the absent required `title` is NOT an
    // edit-time error in the default non-strict mode. Strict mode re-enables it;
    // the strict/non-strict rule itself is covered by classify()'s unit tests.
    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    assert!(
        !diagnostics
            .iter()
            .any(|d| d["code"] == json!("dm.schema.missing_required")),
        "missing_required must be suppressed in non-strict mode: {diagnostics:?}"
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

    // Hover on the `status` key describes the enum, styled per docs/hover.md:
    // the type is bold (never inline code) and the enum members are italic.
    let markup = hover_markup(&mut fixture, uri.as_str(), 4, 2);
    assert!(
        markup.contains("Type: **") && !markup.contains("Type: `"),
        "type must render bold, not inline code: {markup}"
    );
    assert!(
        markup.contains("Values: _draft_, _published_"),
        "enum members must render italic: {markup}"
    );

    fixture.shutdown();
}

/// A document whose inline `$schema` declares a nested inline object
/// (`settings`), exercising completion and hover for keys nested one level
/// under a mapping. `level` is intentionally absent from the authored
/// `settings` mapping so key completion has something to offer.
const NESTED_SCHEMA_DOC: &str = "---\n$schema:\n  settings:\n    mode: enum(dev, prod)\n    path: file\n    level: number\nsettings:\n  mode: dev\n  path: other.md\n---\n\n# Doc\n";

#[test]
fn frontmatter_nested_schema_intelligence() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), NESTED_SCHEMA_DOC).unwrap();
    // A sibling document so the nested `file(...)` value completion has a target.
    std::fs::write(workspace.path().join("other.md"), "# Other\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), NESTED_SCHEMA_DOC);

    // Nested key completion: on the indented `mode` key line (line 7), the
    // enclosing `settings` inline object offers its not-yet-present `level` key.
    let keys = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 2 }
            }),
        )
        .result
        .expect("nested key completions");
    assert!(
        keys.as_array().unwrap().iter().any(|item| item["label"] == json!("level")),
        "expected a nested `level` key completion: {keys:?}"
    );

    // Nested enum value completion: after `mode: ` (line 7) offer the enum
    // members declared on the nested `mode` atom.
    let enum_values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 8 }
            }),
        )
        .result
        .expect("nested enum value completions");
    assert!(
        enum_values.as_array().unwrap().iter().any(|item| item["label"] == json!("prod")),
        "expected a nested enum value completion: {enum_values:?}"
    );

    // Nested `file(...)` value completion: after `path: ` (line 8) offer the
    // sibling workspace document.
    let file_values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 8, "character": 8 }
            }),
        )
        .result
        .expect("nested file value completions");
    assert!(
        file_values.as_array().unwrap().iter().any(|item| item["label"] == json!("other.md")),
        "expected a nested file-path value completion: {file_values:?}"
    );

    // Nested hover: on the `mode` key (line 7) describes the enum, resolved
    // through the `settings` inline object.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 4 }
            }),
        )
        .result
        .expect("nested hover");
    assert!(
        hover["contents"]["value"].as_str().unwrap_or_default().contains("dev"),
        "nested hover should describe the enum: {hover:?}"
    );

    fixture.shutdown();
}

/// A document whose inline `$schema` declares a top-level `file` property and a
/// nested inline object (`settings`) with its own `file` property, so both a
/// top-level and a nested frontmatter value are `file(...)`-typed and navigable.
const FILE_NAV_DOC: &str = "---\n$schema:\n  home: file\n  settings:\n    doc: file\nhome: top.md\nsettings:\n  doc: nested.md\n---\n\n# Doc\n";

#[test]
fn frontmatter_nested_file_navigation() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), FILE_NAV_DOC).unwrap();
    std::fs::write(workspace.path().join("top.md"), "# Top\n").unwrap();
    std::fs::write(workspace.path().join("nested.md"), "# Nested\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), FILE_NAV_DOC);

    // definition on the NESTED `settings.doc` value (line 7, `doc: nested.md`)
    // resolves through the `settings` inline object to nested.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 10 }
            }),
        )
        .result
        .expect("nested definition");
    let locations = definition.as_array().expect("definition array");
    assert_eq!(locations.len(), 1, "nested file definition: {locations:?}");
    assert!(
        locations[0]["uri"].as_str().unwrap().ends_with("nested.md"),
        "nested definition should reach nested.md: {locations:?}"
    );

    // The existing top-level `file(...)` navigation (line 5, `home: top.md`)
    // still resolves through the same code path.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 5, "character": 8 }
            }),
        )
        .result
        .expect("top-level definition");
    let locations = definition.as_array().expect("definition array");
    assert!(
        locations.iter().any(|loc| loc["uri"].as_str().unwrap().ends_with("top.md")),
        "top-level definition should reach top.md: {locations:?}"
    );

    // documentLink yields a link over the nested value's span (targeting
    // nested.md) alongside the top-level one.
    let links = fixture
        .request(
            "textDocument/documentLink",
            json!({ "textDocument": { "uri": uri.as_str() } }),
        )
        .result
        .expect("links");
    let links = links.as_array().expect("link array");
    let nested_link = links.iter().find(|link| {
        link["target"].as_str().is_some_and(|target| target.ends_with("nested.md"))
    });
    let nested_link = nested_link.expect("a document link over the nested file value");
    // The link's range sits on the nested value line (line 7), not a stray line.
    assert_eq!(nested_link["range"]["start"]["line"], json!(7), "{nested_link:?}");
    assert!(
        links
            .iter()
            .any(|link| link["target"].as_str().is_some_and(|t| t.ends_with("top.md"))),
        "top-level file link must still be produced: {links:?}"
    );

    fixture.shutdown();
}

/// A document whose inline `$schema` types `license` as `file` and whose value
/// is a bare extensionless filename (`LICENSE`). A dot/slash heuristic would drop
/// it; once the schema confirms the `file` type it must navigate.
const EXTENSIONLESS_FILE_DOC: &str =
    "---\n$schema:\n  license: file\nlicense: LICENSE\n---\n\n# Doc\n";

#[test]
fn frontmatter_extensionless_file_navigation() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), EXTENSIONLESS_FILE_DOC).unwrap();
    // The extensionless target must exist so resolution has something to reach.
    std::fs::write(workspace.path().join("LICENSE"), "All rights reserved.\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), EXTENSIONLESS_FILE_DOC);

    // definition on the `license` value (line 3, `license: LICENSE`) resolves to
    // the extensionless LICENSE file.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 3, "character": 12 }
            }),
        )
        .result
        .expect("extensionless definition");
    let locations = definition.as_array().expect("definition array");
    assert_eq!(locations.len(), 1, "extensionless file definition: {locations:?}");
    assert!(
        locations[0]["uri"].as_str().unwrap().ends_with("LICENSE"),
        "definition should reach the LICENSE file: {locations:?}"
    );

    // documentLink yields a link over the value's span targeting LICENSE.
    let links = fixture
        .request(
            "textDocument/documentLink",
            json!({ "textDocument": { "uri": uri.as_str() } }),
        )
        .result
        .expect("links");
    let links = links.as_array().expect("link array");
    let license_link = links
        .iter()
        .find(|link| link["target"].as_str().is_some_and(|target| target.ends_with("LICENSE")))
        .expect("a document link over the extensionless file value");
    assert_eq!(license_link["range"]["start"]["line"], json!(3), "{license_link:?}");

    fixture.shutdown();
}

/// A document whose inline `$schema` declares `asset` as a property-level union
/// whose completable arm (`file`) is *second*. The single-atom `primary_atom`
/// path would resolve only the first (`string`) arm and offer no completion, so
/// this exercises capability-specific arm selection for value completion.
const UNION_VALUE_DOC: &str =
    "---\n$schema:\n  asset:\n    - string\n    - file\nasset: \n---\n\n# Doc\n";

#[test]
fn frontmatter_union_value_completion() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), UNION_VALUE_DOC).unwrap();
    // A sibling document so the `file(...)` arm's value completion has a target.
    std::fs::write(workspace.path().join("other.md"), "# Other\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), UNION_VALUE_DOC);

    // Value completion after `asset: ` (line 5) offers the sibling document,
    // driven by the union's *second* (`file`) arm.
    let values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 5, "character": 7 }
            }),
        )
        .result
        .expect("value completions");
    assert!(
        values.as_array().unwrap().iter().any(|item| item["label"] == json!("other.md")),
        "expected a file-path value completion from the second union arm: {values:?}"
    );

    fixture.shutdown();
}

/// A document whose inline `$schema` declares `home` as a property-level union
/// whose `file` arm is *second*, so a file value is only navigable if
/// navigation consults every arm rather than just the first.
const UNION_FILE_NAV_DOC: &str =
    "---\n$schema:\n  home:\n    - string\n    - file\nhome: top.md\n---\n\n# Doc\n";

#[test]
fn frontmatter_union_file_navigation() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), UNION_FILE_NAV_DOC).unwrap();
    std::fs::write(workspace.path().join("top.md"), "# Top\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), UNION_FILE_NAV_DOC);

    // definition on `home: top.md` (line 5) resolves through the second union
    // (`file`) arm to top.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 5, "character": 8 }
            }),
        )
        .result
        .expect("definition");
    let locations = definition.as_array().expect("definition array");
    assert!(
        locations.iter().any(|loc| loc["uri"].as_str().unwrap().ends_with("top.md")),
        "union file arm definition should reach top.md: {locations:?}"
    );

    // documentLink produces a link over the value targeting top.md.
    let links = fixture
        .request("textDocument/documentLink", json!({ "textDocument": { "uri": uri.as_str() } }))
        .result
        .expect("links");
    assert!(
        links
            .as_array()
            .expect("link array")
            .iter()
            .any(|link| link["target"].as_str().is_some_and(|t| t.ends_with("top.md"))),
        "union file arm should produce a document link: {links:?}"
    );

    fixture.shutdown();
}

/// A document whose inline `$schema` declares `settings` as a property-level
/// union whose inline-object arm is *second*. Nested-key intelligence must
/// descend that arm even though the first arm (`string`) is not an object.
const UNION_NESTED_DOC: &str = "---\n$schema:\n  settings:\n    - string\n    - mode: enum(dev, prod)\n      path: file\n      level: number\nsettings:\n  mode: dev\n  path: nested.md\n---\n\n# Doc\n";

#[test]
fn frontmatter_union_nested_schema_intelligence() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), UNION_NESTED_DOC).unwrap();
    std::fs::write(workspace.path().join("nested.md"), "# Nested\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), UNION_NESTED_DOC);

    // Nested key completion: on the `mode` line (line 8), the second-arm inline
    // object offers its not-yet-present `level` key.
    let keys = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 8, "character": 2 }
            }),
        )
        .result
        .expect("nested key completions");
    assert!(
        keys.as_array().unwrap().iter().any(|item| item["label"] == json!("level")),
        "expected a nested `level` key completion through the union arm: {keys:?}"
    );

    // Nested enum value completion: after `mode: ` (line 8) offers the enum
    // members declared on the second-arm inline object's `mode` atom.
    let enum_values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 8, "character": 8 }
            }),
        )
        .result
        .expect("nested enum value completions");
    assert!(
        enum_values.as_array().unwrap().iter().any(|item| item["label"] == json!("prod")),
        "expected a nested enum value completion through the union arm: {enum_values:?}"
    );

    // Nested `file(...)` value completion: after `path: ` (line 9) offers the
    // sibling document.
    let file_values = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 9, "character": 8 }
            }),
        )
        .result
        .expect("nested file value completions");
    assert!(
        file_values.as_array().unwrap().iter().any(|item| item["label"] == json!("nested.md")),
        "expected a nested file-path value completion through the union arm: {file_values:?}"
    );

    // Nested definition on `path: nested.md` (line 9) resolves through the
    // union's inline-object arm to nested.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 9, "character": 10 }
            }),
        )
        .result
        .expect("nested definition");
    assert!(
        definition
            .as_array()
            .expect("definition array")
            .iter()
            .any(|loc| loc["uri"].as_str().unwrap().ends_with("nested.md")),
        "nested union-arm definition should reach nested.md: {definition:?}"
    );

    fixture.shutdown();
}

#[test]
fn claudine_extension_is_pure_config() {
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
fn cancelled_request_answers_request_cancelled() {
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

/// Layer-3 Darkmatter DSL overlay: directive hover + navigation, interpolation
/// hover with a static frontmatter-backed value, and the DSL diagnostic family
/// (unknown directive, broken transclusion, unknown fenced language).
const DSL_DOC: &str = "---\ntitle: Guide\n---\n\n# Guide\n\n::file ./intro.md\n\n::file ./missing.md\n\n::frobnicate x\n\nHello {{ title }}.\n\n```nosuchlang\ncode\n```\n";
const INTRO_DOC: &str = "# Intro\n\nWelcome.\n";

#[test]
fn dsl_overlay_navigation_hover_and_diagnostics() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("guide.md"), DSL_DOC).unwrap();
    std::fs::write(workspace.path().join("intro.md"), INTRO_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));

    let guide_uri = url::Url::from_file_path(workspace.path().join("guide.md")).unwrap();
    let intro_uri = url::Url::from_file_path(workspace.path().join("intro.md")).unwrap();
    open(&fixture, intro_uri.as_str(), INTRO_DOC);
    open(&fixture, guide_uri.as_str(), DSL_DOC);

    // Directive hover on the `::file ./intro.md` target (line 6) describes the
    // directive and resolves its target.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": guide_uri.as_str() },
                "position": { "line": 6, "character": 10 }
            }),
        )
        .result
        .expect("hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(hover_text.contains("::file"), "directive hover: {hover_text}");
    assert!(hover_text.contains("intro.md"), "resolved target: {hover_text}");

    // Definition on the same target jumps into intro.md.
    let definition = fixture
        .request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": guide_uri.as_str() },
                "position": { "line": 6, "character": 10 }
            }),
        )
        .result
        .expect("definition");
    let locations = definition.as_array().expect("definition array");
    assert!(
        locations.iter().any(|loc| loc["uri"] == json!(intro_uri.as_str())),
        "definition should reach intro.md: {locations:?}"
    );

    // Interpolation hover on `{{ title }}` (line 12) shows the static value.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": guide_uri.as_str() },
                "position": { "line": 12, "character": 9 }
            }),
        )
        .result
        .expect("interpolation hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(hover_text.contains("Guide"), "static value from frontmatter: {hover_text}");

    // Diagnostics: unknown directive, broken transclusion, unknown fence language.
    let diagnostics = fixture.wait_for_diagnostics(guide_uri.as_str());
    let codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert!(codes.contains(&"dm.directive.unknown"), "codes: {codes:?}");
    assert!(codes.contains(&"dm.transclusion.broken_path"), "codes: {codes:?}");
    assert!(codes.contains(&"dm.fence.unknown_language"), "codes: {codes:?}");

    fixture.shutdown();
}

/// Compose-parity: a directive-name completion after `::` offers the catalog.
#[test]
fn dsl_directive_name_completion() {
    let workspace = tempfile::tempdir().unwrap();
    let text = "# Doc\n\n::fi\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let completion = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 4 }
            }),
        )
        .result
        .expect("completion");
    let items = completion.as_array().expect("completion array");
    let labels: Vec<&str> = items.iter().filter_map(|item| item["label"].as_str()).collect();
    assert!(labels.contains(&"::file"), "labels: {labels:?}");
    assert!(labels.contains(&"::file-links"), "labels: {labels:?}");

    fixture.shutdown();
}

/// Compose-parity (no false positives): a document whose directives,
/// interpolation, block pairing, and fenced language are all valid emits **no**
/// Layer-3 DSL diagnostic. Only brokenness `md compose` would also reject is
/// flagged.
const VALID_DSL_DOC: &str = "---\ntitle: Guide\n---\n\n# Guide\n\n::file ./intro.md\n\n::block when=\"true\"\ncontent\n::end-block\n\nHello {{ title }} on {{ ctx.today }}.\n\n```rust\nfn main() {}\n```\n";

#[test]
fn dsl_valid_document_has_no_dsl_diagnostics() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("guide.md"), VALID_DSL_DOC).unwrap();
    std::fs::write(workspace.path().join("intro.md"), INTRO_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let guide_uri = url::Url::from_file_path(workspace.path().join("guide.md")).unwrap();
    let intro_uri = url::Url::from_file_path(workspace.path().join("intro.md")).unwrap();
    open(&fixture, intro_uri.as_str(), INTRO_DOC);
    open(&fixture, guide_uri.as_str(), VALID_DSL_DOC);

    let diagnostics = fixture.wait_for_diagnostics(guide_uri.as_str());
    let dsl_codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .filter(|code| {
            code.starts_with("dm.directive.")
                || code.starts_with("dm.transclusion.")
                || code.starts_with("dm.expression.")
                || code.starts_with("dm.security.")
                || code.starts_with("dm.fence.")
        })
        .collect();
    assert!(dsl_codes.is_empty(), "valid doc must have no DSL diagnostics: {dsl_codes:?}");

    fixture.shutdown();
}

/// A schema-declared-but-unset property is a valid body interpolation, and
/// `json5` / `mermaid` are recognized fenced languages: none of the three emit a
/// DSL diagnostic. The `spec` property is declared by the inline `$schema` (a
/// caller-supplied `file` parameter) yet never set in the document frontmatter,
/// exactly the compose-time-merge pattern; `{{ spec }}` must not be flagged as
/// an unknown identifier, and its hover surfaces the schema type/description.
const SCHEMA_PROPERTY_DOC: &str = "---\ntitle: Review\n$schema:\n  spec: file(required) -> the specification file being reviewed\n---\n\n# Review\n\nReviewing {{ spec }}.\n\n```json5\n{ id: \"one\" }\n```\n\n```mermaid\ngraph TD; A-->B;\n```\n";

#[test]
fn schema_declared_property_and_json5_mermaid_have_no_dsl_diagnostics() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("review.md"), SCHEMA_PROPERTY_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("review.md")).unwrap();
    open(&fixture, uri.as_str(), SCHEMA_PROPERTY_DOC);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let codes: Vec<&str> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .collect();
    assert!(
        !codes.contains(&"dm.expression.unknown_identifier"),
        "schema-declared `spec` must not be flagged: {codes:?}"
    );
    assert!(
        !codes.contains(&"dm.fence.unknown_language"),
        "json5 and mermaid are recognized: {codes:?}"
    );

    // Hover on `{{ spec }}` (line 8, `spec` starts at character 13) surfaces the
    // schema type and description even though the property is unset.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 8, "character": 14 }
            }),
        )
        .result
        .expect("interpolation hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(hover_text.contains("file"), "schema type in hover: {hover_text}");
    assert!(
        hover_text.contains("the specification file being reviewed"),
        "schema description in hover: {hover_text}"
    );
    // The name appears once (in the `**Expression**` header); the schema details
    // carry no second `**`spec`**` heading.
    assert!(
        !hover_text.contains("**`spec`**"),
        "property name must not render twice: {hover_text}"
    );

    fixture.shutdown();
}

/// Layer-3 shell awareness reaches into `::shell-block` bodies: a disallowed
/// command inside the block is flagged with `dm.security.disallowed_command`,
/// ranged on the offending body line, while a benign sibling line is not — and
/// hovering a body command line shows the command with a policy verdict.
const SHELL_BLOCK_DOC: &str =
    "---\ntitle: Shell\n---\n\n# Shell\n\n::shell-block\necho hello\nrm -rf /tmp/danger\n::end-block\n";

#[test]
fn dsl_shell_block_body_disallowed_command_is_diagnosed() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), SHELL_BLOCK_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), SHELL_BLOCK_DOC);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let security: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.security.disallowed_command"))
        .collect();
    assert_eq!(security.len(), 1, "one shell-block line is disallowed: {diagnostics:?}");
    // Ranged on the `rm -rf` body line (0-indexed line 8), not the benign echo.
    assert_eq!(
        security[0]["range"]["start"]["line"],
        json!(8),
        "diagnostic must sit on the offending body line: {:?}",
        security[0]
    );

    fixture.shutdown();
}

#[test]
fn dsl_shell_block_body_hover_shows_policy_verdict() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), SHELL_BLOCK_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), SHELL_BLOCK_DOC);

    // Hover on the `echo hello` body line (line 7) shows the command + verdict.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 2 }
            }),
        )
        .result
        .expect("shell-block body hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(hover_text.contains("Shell command"), "hover: {hover_text}");
    assert!(hover_text.contains("echo hello"), "hover command: {hover_text}");
    assert!(hover_text.contains("Policy:"), "hover verdict: {hover_text}");

    fixture.shutdown();
}

/// A `::shell-block` whose body lines are all benign emits no security
/// diagnostic — no false positive on valid shell scripts.
const BENIGN_SHELL_BLOCK_DOC: &str =
    "---\ntitle: Shell\n---\n\n# Shell\n\n::shell-block\necho hello\ngit status\n::end-block\n";

#[test]
fn dsl_shell_block_benign_body_has_no_security_diagnostic() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), BENIGN_SHELL_BLOCK_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), BENIGN_SHELL_BLOCK_DOC);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let has_security = diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == json!("dm.security.disallowed_command"));
    assert!(!has_security, "benign shell block must not be flagged: {diagnostics:?}");

    fixture.shutdown();
}

/// A block-quoted `::shell-block` still exposes its real command to policy: the
/// shared library splitter strips the `> ` marker, so a disallowed command
/// inside a quoted block is flagged (a naive line split would classify the `>`
/// marker as the executable and miss it).
const QUOTED_SHELL_BLOCK_DOC: &str =
    "---\ntitle: Shell\n---\n\n# Shell\n\n> ::shell-block\n> rm -rf /tmp/danger\n> ::end-block\n";

#[test]
fn dsl_quoted_shell_block_body_disallowed_command_is_diagnosed() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), QUOTED_SHELL_BLOCK_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), QUOTED_SHELL_BLOCK_DOC);

    let diagnostics = fixture.wait_for_diagnostics(uri.as_str());
    let security: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.security.disallowed_command"))
        .collect();
    assert_eq!(
        security.len(),
        1,
        "quoted shell-block command must be classified past the `>` marker: {diagnostics:?}"
    );

    fixture.shutdown();
}

// ── Phase 10: rename, code actions, formatting ──

/// A VS Code-like client: resource operations, file operations, rename prepare
/// support, and completion — the full editing surface.
fn vscode_like_initialize_params(root: &std::path::Path) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    json!({
        "processId": null,
        "clientInfo": { "name": "Visual Studio Code", "version": "1.90.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": {
                "configuration": true,
                "workspaceEdit": {
                    "documentChanges": true,
                    "resourceOperations": ["create", "rename", "delete"]
                },
                "fileOperations": { "willRename": true }
            },
            "textDocument": {
                "rename": { "prepareSupport": true },
                "codeAction": {},
                "foldingRange": {}
            }
        },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
    })
}

/// Every `newText` in a WorkspaceEdit result, from `documentChanges` or `changes`.
fn collect_new_texts(edit: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(ops) = edit["documentChanges"].as_array() {
        for op in ops {
            if let Some(edits) = op["edits"].as_array() {
                out.extend(edits.iter().filter_map(|e| e["newText"].as_str().map(str::to_string)));
            }
        }
    }
    if let Some(changes) = edit["changes"].as_object() {
        for edits in changes.values() {
            if let Some(edits) = edits.as_array() {
                out.extend(edits.iter().filter_map(|e| e["newText"].as_str().map(str::to_string)));
            }
        }
    }
    out
}

/// Every document URI touched by a WorkspaceEdit result.
fn collect_edit_uris(edit: &Value) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(ops) = edit["documentChanges"].as_array() {
        for op in ops {
            if let Some(uri) = op["textDocument"]["uri"].as_str() {
                out.push(uri.to_string());
            }
        }
    }
    if let Some(changes) = edit["changes"].as_object() {
        out.extend(changes.keys().cloned());
    }
    out
}

#[test]
fn formatting_is_byte_equivalent_to_library_cleanup() {
    let workspace = tempfile::tempdir().unwrap();
    let mut fixture = ClientFixture::start();
    fixture.initialize(vscode_like_initialize_params(workspace.path()));

    let doc_uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    let source = "# Title\nParagraph one.\n## Sub\nMore text.\n";
    open(&fixture, doc_uri.as_str(), source);

    let edits = fixture
        .request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": doc_uri.as_str() },
                "options": { "tabSize": 2, "insertSpaces": true }
            }),
        )
        .result
        .expect("formatting");
    let edits = edits.as_array().expect("formatting edits");
    assert_eq!(edits.len(), 1, "whole-document replacement expected");

    // Spec criterion 8: the edit is exactly the library cleanup output.
    let mut md: darkmatter::markdown::Markdown = source.into();
    md.cleanup();
    let expected = md.as_string();
    assert_eq!(edits[0]["newText"], json!(expected));

    fixture.shutdown();
}

#[test]
fn code_action_creates_missing_wiki_note() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("note.md"), "See [[NewNote]].\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(vscode_like_initialize_params(workspace.path()));

    let note_uri = url::Url::from_file_path(workspace.path().join("note.md")).unwrap();
    open(&fixture, note_uri.as_str(), "See [[NewNote]].\n");

    // The unresolved-target diagnostic drives the create action.
    let diagnostics = fixture.wait_for_diagnostics(note_uri.as_str());
    let unresolved = diagnostics
        .iter()
        .find(|d| d["code"] == json!("wiki.unresolved-target"))
        .expect("unresolved wiki diagnostic")
        .clone();

    let actions = fixture
        .request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": note_uri.as_str() },
                "range": unresolved["range"],
                "context": { "diagnostics": [unresolved] }
            }),
        )
        .result
        .expect("code actions");
    let actions = actions.as_array().expect("action array");
    let creates_note = actions.iter().any(|action| {
        action["edit"]["documentChanges"]
            .as_array()
            .is_some_and(|ops| {
                ops.iter().any(|op| {
                    op["kind"] == json!("create")
                        && op["uri"].as_str().is_some_and(|uri| uri.ends_with("NewNote.md"))
                })
            })
    });
    assert!(creates_note, "expected a create-wiki-note action: {actions:?}");

    fixture.shutdown();
}

const RENAME_DOC_A: &str = "# Overview\n\n[top](#overview)\n";
const RENAME_DOC_B: &str = "See [[doc_a#Overview]].\n";

#[test]
fn heading_rename_rewrites_references() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc_a.md"), RENAME_DOC_A).unwrap();
    std::fs::write(workspace.path().join("doc_b.md"), RENAME_DOC_B).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(vscode_like_initialize_params(workspace.path()));

    let a_uri = url::Url::from_file_path(workspace.path().join("doc_a.md")).unwrap();
    let b_uri = url::Url::from_file_path(workspace.path().join("doc_b.md")).unwrap();
    open(&fixture, b_uri.as_str(), RENAME_DOC_B);
    open(&fixture, a_uri.as_str(), RENAME_DOC_A);

    // prepareRename on the heading returns its text range and current title.
    let prepared = fixture
        .request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 0, "character": 4 }
            }),
        )
        .result
        .expect("prepareRename");
    assert_eq!(prepared["placeholder"], json!("Overview"));

    // rename rewrites the heading, the slug anchor, and the wiki reference.
    let edit = fixture
        .request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": a_uri.as_str() },
                "position": { "line": 0, "character": 4 },
                "newName": "Summary"
            }),
        )
        .result
        .expect("rename edit");
    let new_texts = collect_new_texts(&edit);
    // Text-form links (heading + wiki exact text) get the new title; the
    // Markdown slug anchor gets the new slug.
    assert!(new_texts.contains(&"Summary".to_string()), "{new_texts:?}");
    assert!(new_texts.contains(&"summary".to_string()), "{new_texts:?}");
    let uris = collect_edit_uris(&edit);
    assert!(
        uris.iter().any(|uri| uri.ends_with("doc_b.md")),
        "cross-document wiki reference must be rewritten: {uris:?}"
    );

    fixture.shutdown();
}

#[test]
fn rename_refuses_ambiguous_heading() {
    let workspace = tempfile::tempdir().unwrap();
    let source = "# Same\n\ntext\n\n# Same\n";
    std::fs::write(workspace.path().join("dup.md"), source).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(vscode_like_initialize_params(workspace.path()));

    let uri = url::Url::from_file_path(workspace.path().join("dup.md")).unwrap();
    open(&fixture, uri.as_str(), source);

    // prepareRename refuses (null) on an ambiguous heading.
    let prepared = fixture
        .request(
            "textDocument/prepareRename",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 0, "character": 3 }
            }),
        )
        .result
        .expect("prepareRename result");
    assert_eq!(prepared, Value::Null);

    // rename returns an error rather than a partial edit (spec criterion 9).
    let response = fixture.request(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri.as_str() },
            "position": { "line": 0, "character": 3 },
            "newName": "Renamed"
        }),
    );
    assert!(response.error.is_some(), "ambiguous rename must error");

    fixture.shutdown();
}

#[test]
fn will_rename_files_updates_and_refuses() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("note1.md"), "See [[note2]].\n").unwrap();
    std::fs::write(workspace.path().join("note2.md"), "# Note Two\n").unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(vscode_like_initialize_params(workspace.path()));

    let note1 = url::Url::from_file_path(workspace.path().join("note1.md")).unwrap();
    let note2 = url::Url::from_file_path(workspace.path().join("note2.md")).unwrap();
    open(&fixture, note1.as_str(), "See [[note2]].\n");
    open(&fixture, note2.as_str(), "# Note Two\n");

    // Safe rename: `[[note2]]` becomes `[[renamed]]`.
    let renamed = url::Url::from_file_path(workspace.path().join("renamed.md")).unwrap();
    let edit = fixture
        .request(
            "workspace/willRenameFiles",
            json!({
                "files": [ { "oldUri": note2.as_str(), "newUri": renamed.as_str() } ]
            }),
        )
        .result
        .expect("willRenameFiles edit");
    let new_texts = collect_new_texts(&edit);
    assert!(new_texts.contains(&"renamed".to_string()), "{new_texts:?}");

    // Conflict: renaming onto an existing tracked document refuses (null).
    let response = fixture
        .request(
            "workspace/willRenameFiles",
            json!({
                "files": [ { "oldUri": note2.as_str(), "newUri": note1.as_str() } ]
            }),
        )
        .result
        .expect("willRenameFiles result");
    assert_eq!(response, Value::Null, "rename onto an existing file must refuse");

    fixture.shutdown();
}

// ── Modal-and-autocomplete Phase 4: interpolation assistance end-to-end ──

#[test]
fn initialize_advertises_period_completion_trigger() {
    let mut fixture = ClientFixture::start();
    let result = fixture.initialize(json!({ "processId": null, "capabilities": {} }));

    // D3: `.` joins the trigger set without dropping any existing trigger.
    let triggers = result["capabilities"]["completionProvider"]["triggerCharacters"]
        .as_array()
        .expect("completion trigger characters")
        .clone();
    for trigger in ["/", "(", "#", "."] {
        assert!(
            triggers.contains(&json!(trigger)),
            "`{trigger}` must be an advertised completion trigger: {triggers:?}"
        );
    }

    fixture.shutdown();
}

/// A document whose frontmatter carries a `ctx.packages` key and whose body
/// interpolates the same variable, so the two hover surfaces can be compared.
const CTX_HOVER_DOC: &str =
    "---\nctx:\n  packages: []\n---\n\n# Doc\n\nList: {{ ctx.packages }}\n";

#[test]
fn interpolation_ctx_hover_matches_frontmatter_block() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), CTX_HOVER_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), CTX_HOVER_DOC);

    let block = expressions::format_ctx_hover_block(
        expressions::ctx_descriptor("packages").expect("`packages` is a known context variable"),
    );

    // Interpolation hover on `{{ ctx.packages }}` (line 7): the catalog-derived
    // type and description are present, plus the interpolation-only note.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 12 }
            }),
        )
        .result
        .expect("interpolation hover");
    let interpolation_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(interpolation_text.contains("string[]"), "catalog type: {interpolation_text}");
    assert!(
        interpolation_text.contains(&block),
        "catalog-backed block must be the shared formatter's bytes: {interpolation_text}"
    );
    assert!(
        interpolation_text.contains("_compose_ time"),
        "passive compose-time note must be appended: {interpolation_text}"
    );
    // The hover range is the complete `{{ ... }}` expression.
    assert_eq!(hover["range"]["start"], json!({ "line": 7, "character": 6 }));
    assert_eq!(hover["range"]["end"], json!({ "line": 7, "character": 24 }));

    // Frontmatter hover on the `packages` key under `ctx:` (line 2): the same
    // catalog-backed block byte-for-byte, without the compose-time note.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 3 }
            }),
        )
        .result
        .expect("frontmatter ctx hover");
    let frontmatter_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert_eq!(
        frontmatter_text.trim_end(),
        block,
        "frontmatter ctx hover must be exactly the shared catalog block"
    );
    assert!(
        !frontmatter_text.contains("compose"),
        "the compose-time note is interpolation-specific: {frontmatter_text}"
    );

    fixture.shutdown();
}

/// A document whose body interpolates `ctx.packages` reached through index
/// access (`ctx.packages[0]`), so the D2 hover contract is verified end-to-end
/// for a ctx variable that is the root of an `Index` node — not just a direct
/// `ctx.packages` reference.
const INDEX_CTX_DOC: &str =
    "---\nctx:\n  packages: []\n---\n\n# Doc\n\nItem: {{ ctx.packages[0] }}\n";

#[test]
fn interpolation_ctx_hover_surfaces_through_index_access() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), INDEX_CTX_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), INDEX_CTX_DOC);

    let block = expressions::format_ctx_hover_block(
        expressions::ctx_descriptor("packages").expect("`packages` is a known context variable"),
    );

    // Hover on the `packages` portion of `ctx.packages[0]` (line 7). The body
    // line is `Item: {{ ctx.packages[0] }}`: `Item: ` is 6 columns, `{{ ` lands
    // at columns 6-8, so `ctx` begins at column 9 and `packages` at column 13.
    // The negotiated encoding is UTF-8, so byte offsets equal character columns.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 7, "character": 13 }
            }),
        )
        .result
        .expect("interpolation hover");
    let text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(text.contains("string[]"), "catalog type surfaces: {text}");
    assert!(
        text.contains(&block),
        "catalog-backed block must be the shared formatter's bytes: {text}"
    );
    assert!(
        text.contains("_compose_ time"),
        "passive compose-time note must be appended: {text}"
    );

    // The hover range spans the complete `{{ ctx.packages[0] }}` expression
    // (D2): `{{` opens at column 6 and `}}` closes at column 27 (exclusive).
    assert_eq!(hover["range"]["start"], json!({ "line": 7, "character": 6 }));
    assert_eq!(hover["range"]["end"], json!({ "line": 7, "character": 27 }));

    fixture.shutdown();
}

/// An astral character (💡, two UTF-16 units) precedes the open interpolation,
/// so the negotiated UTF-16 position path is exercised: `ctx.pa` spans UTF-16
/// columns 6..12 even though its byte span starts at 8.
const ASTRAL_CTX_COMPLETION_DOC: &str = "# Doc\n\n💡 {{ ctx.pa\n";

#[test]
fn ctx_completion_shape_under_utf16_astral_prefix() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), ASTRAL_CTX_COMPLETION_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    // VS Code-like client: offers UTF-16 only, so positions are UTF-16 columns.
    fixture.initialize(vscode_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), ASTRAL_CTX_COMPLETION_DOC);

    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 12 }
            }),
        )
        .result
        .expect("completions");
    let items = completions.as_array().expect("completion array");
    let packages = items
        .iter()
        .find(|item| item["label"] == json!("ctx.packages"))
        .expect("`ctx.pa` offers `ctx.packages`");

    // D3: `detail` is the rendered type, `documentation` is eager Markdown.
    let descriptor = expressions::ctx_descriptor("packages").unwrap();
    assert_eq!(packages["kind"], json!(6), "kind VARIABLE: {packages:?}");
    assert_eq!(packages["detail"], json!("string[]"));
    assert_eq!(packages["documentation"]["kind"], json!("markdown"));
    assert_eq!(packages["documentation"]["value"], json!(descriptor.description));
    // The eager `textEdit` replaces exactly the typed token, in UTF-16 columns
    // (start 6, not the byte column 8 — the astral 💡 counts as two units).
    assert_eq!(packages["textEdit"]["newText"], json!("ctx.packages"));
    assert_eq!(packages["textEdit"]["range"]["start"], json!({ "line": 2, "character": 6 }));
    assert_eq!(packages["textEdit"]["range"]["end"], json!({ "line": 2, "character": 12 }));

    fixture.shutdown();
}

const FUNCTION_COMPLETION_DOC: &str = "# Doc\n\nValue: {{ len\n";

#[test]
fn function_completion_shape() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), FUNCTION_COMPLETION_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), FUNCTION_COMPLETION_DOC);

    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 13 }
            }),
        )
        .result
        .expect("completions");
    let items = completions.as_array().expect("completion array");
    let length = items
        .iter()
        .find(|item| item["textEdit"]["newText"] == json!("length"))
        .expect("`len` offers `length` with bare-name insertion");

    // D4: untyped signature label, typed-signature detail (fallible → `| error`),
    // eager Markdown documentation, bare-name insertion.
    let descriptor = expressions::function_descriptor("length").unwrap();
    assert_eq!(length["label"], json!(descriptor.signature));
    assert_eq!(length["detail"], json!(descriptor.typed_signature()));
    assert!(
        length["detail"].as_str().unwrap().ends_with("| error"),
        "fallible typed signature must carry `| error`: {length:?}"
    );
    assert_eq!(length["documentation"]["kind"], json!("markdown"));
    assert_eq!(length["documentation"]["value"], json!(descriptor.description));
    assert_eq!(length["textEdit"]["range"]["start"], json!({ "line": 2, "character": 10 }));
    assert_eq!(length["textEdit"]["range"]["end"], json!({ "line": 2, "character": 13 }));

    fixture.shutdown();
}

const FUNCTION_HOVER_DOC: &str = "# Doc\n\nA: {{ as_csv(items) }} and {{ mystery(items) }}\n";

#[test]
fn function_call_hover_known_and_unknown() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), FUNCTION_HOVER_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), FUNCTION_HOVER_DOC);

    // Hover inside `as_csv(...)` renders the typed signature + description.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 8 }
            }),
        )
        .result
        .expect("function hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    let as_csv = expressions::function_descriptor("as_csv").unwrap();
    assert!(
        hover_text.contains(&expressions::format_function_block(as_csv)),
        "known function renders the catalog block: {hover_text}"
    );
    // The hover range remains the complete `{{ ... }}` expression.
    assert_eq!(hover["range"]["start"], json!({ "line": 2, "character": 3 }));
    assert_eq!(hover["range"]["end"], json!({ "line": 2, "character": 22 }));

    // Hover inside `mystery(...)` keeps the generic parsed-expression hover.
    let hover = fixture
        .request(
            "textDocument/hover",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 32 }
            }),
        )
        .result
        .expect("unknown-function hover");
    let hover_text = hover["contents"]["value"].as_str().unwrap_or_default();
    assert!(
        hover_text.starts_with("**Expression**"),
        "unknown function keeps the generic hover: {hover_text}"
    );
    assert!(
        !hover_text.contains("**`mystery"),
        "no catalog block may be synthesized for an unknown function: {hover_text}"
    );

    fixture.shutdown();
}

const PROSE_PERIOD_DOC: &str = "# Doc\n\nplain prose ctx.\n";

#[test]
fn period_trigger_outside_interpolation_offers_nothing() {
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("doc.md"), PROSE_PERIOD_DOC).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(neovim_like_initialize_params(workspace.path()));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), PROSE_PERIOD_DOC);

    // The advertised `.` trigger fires in ordinary prose: the DSL provider's
    // open-interpolation guard offers nothing (D3/criterion 4).
    let completions = fixture
        .request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri.as_str() },
                "position": { "line": 2, "character": 16 },
                "context": { "triggerKind": 2, "triggerCharacter": "." }
            }),
        )
        .result
        .expect("completion result");
    assert_eq!(
        completions,
        json!([]),
        "a period outside an open interpolation must produce no completion items"
    );

    fixture.shutdown();
}

// ── Phase 5: semantic tokens end-to-end ──
//
// The V1 legend indices, mirrored here as the wire contract the session tests
// assert against (kept in lock-step with `semantic_legend::legend`).
const TT_MACRO: u32 = 0;
const TT_PROPERTY: u32 = 3;
const TT_STRING: u32 = 4;
const TM_INTERPOLATION: u32 = 1 << 0;
const TM_INERT: u32 = 1 << 1;
const TM_DIRECTIVE: u32 = 1 << 2;
const TM_CLOSER: u32 = 1 << 3;
const TM_WIKI: u32 = 1 << 4;

/// One decoded absolute semantic token: `(line, character, length, type, modifiers)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tok {
    line: u32,
    character: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

fn tok(line: u32, character: u32, length: u32, token_type: u32, modifiers: u32) -> Tok {
    Tok { line, character, length, token_type, modifiers }
}

/// A minimal valid `textDocument.semanticTokens` client capability.
fn semantic_tokens_client_capability() -> Value {
    json!({
        "requests": { "full": true, "range": true },
        "tokenTypes": [],
        "tokenModifiers": [],
        "formats": []
    })
}

/// Initialize params for a semantic-token-capable client. `encoding` is the only
/// position encoding the client offers, so the negotiated encoding is
/// deterministic; `refresh` advertises `workspace.semanticTokens.refreshSupport`.
fn semantic_capable_params(root: &std::path::Path, encoding: &str, refresh: bool) -> Value {
    let mut params = neovim_like_initialize_params(root);
    params["capabilities"]["general"]["positionEncodings"] = json!([encoding]);
    params["capabilities"]["textDocument"]["semanticTokens"] = semantic_tokens_client_capability();
    if refresh {
        params["capabilities"]["workspace"]["semanticTokens"] = json!({ "refreshSupport": true });
    }
    params
}

/// Reverses the LSP relative-delta encoding in a semantic-token response back to
/// absolute `(line, character, length, type, modifiers)` tuples.
fn decode_tokens(result: &Value) -> Vec<Tok> {
    let data = result["data"].as_array().expect("semantic-token data array");
    assert_eq!(data.len() % 5, 0, "token data must be 5-integer groups: {data:?}");
    let mut out = Vec::new();
    let mut line = 0u32;
    let mut character = 0u32;
    for group in data.chunks(5) {
        let field = |index: usize| group[index].as_u64().expect("token integer") as u32;
        let (delta_line, delta_start) = (field(0), field(1));
        if delta_line == 0 {
            character += delta_start;
        } else {
            line += delta_line;
            character = delta_start;
        }
        out.push(Tok {
            line,
            character,
            length: field(2),
            token_type: field(3),
            modifiers: field(4),
        });
    }
    out
}

fn semantic_full(fixture: &mut ClientFixture, uri: &str) -> Vec<Tok> {
    let result = fixture
        .request("textDocument/semanticTokens/full", json!({ "textDocument": { "uri": uri } }))
        .result
        .expect("full semantic tokens");
    decode_tokens(&result)
}

fn semantic_range(
    fixture: &mut ClientFixture,
    uri: &str,
    start: (u32, u32),
    end: (u32, u32),
) -> Vec<Tok> {
    let result = fixture
        .request(
            "textDocument/semanticTokens/range",
            json!({
                "textDocument": { "uri": uri },
                "range": {
                    "start": { "line": start.0, "character": start.1 },
                    "end": { "line": end.0, "character": end.1 }
                }
            }),
        )
        .result
        .expect("range semantic tokens");
    decode_tokens(&result)
}

/// Every semantic-token response must be strictly `(line, character)`-ordered
/// with no overlapping decoded span (spec criteria 4 and 10).
fn assert_ordered_nonoverlapping(tokens: &[Tok]) {
    for pair in tokens.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        assert!(
            (a.line, a.character) < (b.line, b.character),
            "tokens not strictly (line, char)-ordered: {a:?} then {b:?}"
        );
        if a.line == b.line {
            assert!(a.character + a.length <= b.character, "tokens overlap: {a:?} then {b:?}");
        }
    }
}

#[test]
fn semantic_tokens_capability_is_gated_on_client_support() {
    // Criterion 6: only a client advertising `textDocument.semanticTokens` sees
    // the provider capability, and it publishes the frozen V1 legend with full +
    // range and no delta support.
    let workspace = tempfile::tempdir().unwrap();

    let mut capable = ClientFixture::start();
    let result = capable.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let provider = result["capabilities"]["semanticTokensProvider"].clone();
    assert_eq!(
        provider["legend"]["tokenTypes"],
        json!(["macro", "function", "variable", "property", "string", "number", "operator"]),
        "capable client must receive the frozen token-type legend: {provider}"
    );
    assert_eq!(
        provider["legend"]["tokenModifiers"],
        json!([
            "interpolation",
            "inert",
            "directive",
            "closer",
            "wiki",
            "injected",
            "defaultLibrary",
            "readonly"
        ]),
        "capable client must receive the frozen modifier legend: {provider}"
    );
    assert_eq!(provider["full"], json!(true), "full support advertised: {provider}");
    assert_eq!(provider["range"], json!(true), "range support advertised: {provider}");
    assert!(provider["delta"].is_null(), "v1 advertises no delta support: {provider}");
    capable.shutdown();

    // An incapable client (default Neovim params carry no `semanticTokens`
    // capability) sees no provider entry at all.
    let mut incapable = ClientFixture::start();
    let result = incapable.initialize(neovim_like_initialize_params(workspace.path()));
    assert!(
        result["capabilities"]["semanticTokensProvider"].is_null(),
        "incapable client must not see a semantic-token provider: {result}"
    );
    incapable.shutdown();
}

#[test]
fn semantic_tokens_full_interpolations_ordinary_inert_and_multiline() {
    // Criteria 1 and 4: an ordinary `{{ }}` interpolation, a `{{{ }}}` inert
    // literal, and a multi-line interpolation split into one token per line.
    let workspace = tempfile::tempdir().unwrap();
    let text = "{{ title }} and {{{ verbatim }}}\n{{ a\nb }}\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(
        full,
        vec![
            tok(0, 0, 11, TT_MACRO, TM_INTERPOLATION),
            tok(0, 16, 16, TT_MACRO, TM_INTERPOLATION | TM_INERT),
            tok(1, 0, 4, TT_MACRO, TM_INTERPOLATION),
            tok(2, 0, 4, TT_MACRO, TM_INTERPOLATION),
        ]
    );
    assert_ordered_nonoverlapping(&full);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_full_directive_token_classes() {
    // Criterion 2: keyword, structured target, option key, and option value, plus
    // the `closer` modifier on a structural closer.
    let workspace = tempfile::tempdir().unwrap();
    let text = "::file ./doc.md when=env.DEBUG\n::end-block\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(
        full,
        vec![
            tok(0, 0, 6, TT_MACRO, TM_DIRECTIVE),      // ::file
            tok(0, 7, 8, TT_STRING, TM_DIRECTIVE),     // ./doc.md
            tok(0, 16, 4, TT_PROPERTY, TM_DIRECTIVE),  // when
            tok(0, 21, 9, TT_STRING, TM_DIRECTIVE),    // env.DEBUG
            tok(1, 0, 11, TT_MACRO, TM_DIRECTIVE | TM_CLOSER), // ::end-block
        ]
    );
    assert_ordered_nonoverlapping(&full);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_full_wiki_segments_identical_for_resolved_and_unresolved() {
    // Criterion 3: bracket/separator machinery is `macro.wiki`, inner segments are
    // `string.wiki`, and the token shape is identical whether the target resolves.
    let workspace = tempfile::tempdir().unwrap();
    std::fs::write(workspace.path().join("Target.md"), "# Target\n").unwrap();
    let text = "[[doc#H|Alias]]\n\n[[Target]] and [[Missing]]\n";
    std::fs::write(workspace.path().join("Source.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("Source.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_ordered_nonoverlapping(&full);

    // `[[doc#H|Alias]]`: brackets and `#`/`|` separators macro.wiki, segments string.wiki.
    let line0: Vec<Tok> = full.iter().copied().filter(|token| token.line == 0).collect();
    assert_eq!(
        line0,
        vec![
            tok(0, 0, 2, TT_MACRO, TM_WIKI),  // [[
            tok(0, 2, 3, TT_STRING, TM_WIKI), // doc
            tok(0, 5, 1, TT_MACRO, TM_WIKI),  // #
            tok(0, 6, 1, TT_STRING, TM_WIKI), // H
            tok(0, 7, 1, TT_MACRO, TM_WIKI),  // |
            tok(0, 8, 5, TT_STRING, TM_WIKI), // Alias
            tok(0, 13, 2, TT_MACRO, TM_WIKI), // ]]
        ]
    );

    // The resolved `[[Target]]` and the unresolved `[[Missing]]` share a shape.
    let shape = |tokens: &[Tok]| {
        tokens.iter().map(|token| (token.token_type, token.modifiers)).collect::<Vec<_>>()
    };
    let resolved: Vec<Tok> =
        full.iter().copied().filter(|token| token.line == 2 && token.character < 11).collect();
    let unresolved: Vec<Tok> =
        full.iter().copied().filter(|token| token.line == 2 && token.character >= 15).collect();
    assert_eq!(
        shape(&resolved),
        shape(&unresolved),
        "resolved and unresolved wiki links must share a token shape: {resolved:?} vs {unresolved:?}"
    );

    fixture.shutdown();
}

#[test]
fn semantic_tokens_encoding_utf8_vs_utf16_on_non_ascii() {
    // Criterion 4: the same interpolation lands at a different column in UTF-8 vs
    // UTF-16 because the preceding `é` is two bytes but one UTF-16 unit.
    let workspace = tempfile::tempdir().unwrap();
    let text = "é {{ x }}\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();

    let mut utf8 = ClientFixture::start();
    utf8.initialize(semantic_capable_params(workspace.path(), "utf-8", false));
    open(&utf8, uri.as_str(), text);
    let utf8_tokens = semantic_full(&mut utf8, uri.as_str());
    // Byte column: "é " is 3 bytes.
    assert_eq!(utf8_tokens, vec![tok(0, 3, 7, TT_MACRO, TM_INTERPOLATION)]);
    utf8.shutdown();

    let mut utf16 = ClientFixture::start();
    utf16.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    open(&utf16, uri.as_str(), text);
    let utf16_tokens = semantic_full(&mut utf16, uri.as_str());
    // UTF-16 column: "é " is 2 units.
    assert_eq!(utf16_tokens, vec![tok(0, 2, 7, TT_MACRO, TM_INTERPOLATION)]);
    utf16.shutdown();
}

#[test]
fn semantic_tokens_crlf_multiline_splits_per_line() {
    // Criterion 4: a multi-line interpolation in a CRLF document splits per line,
    // excluding the line terminators.
    let workspace = tempfile::tempdir().unwrap();
    let text = "{{ a\r\nb }}\r\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(
        full,
        vec![
            tok(0, 0, 4, TT_MACRO, TM_INTERPOLATION), // {{ a
            tok(1, 0, 4, TT_MACRO, TM_INTERPOLATION), // b }}
        ]
    );
    assert_ordered_nonoverlapping(&full);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_range_is_full_intersected() {
    // Criterion 9: a range response is exactly the full response intersected and
    // clipped to the requested half-open range — including a boundary inside a
    // token and a range that crosses a line ending.
    let workspace = tempfile::tempdir().unwrap();
    let text = "{{aaa}} {{bbb}}\nzz {{ccc}}\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(
        full,
        vec![
            tok(0, 0, 7, TT_MACRO, TM_INTERPOLATION),
            tok(0, 8, 7, TT_MACRO, TM_INTERPOLATION),
            tok(1, 3, 7, TT_MACRO, TM_INTERPOLATION),
        ]
    );

    // A single-line range whose start lands inside the first token.
    let single = semantic_range(&mut fixture, uri.as_str(), (0, 3), (0, 10));
    assert_eq!(
        single,
        vec![
            tok(0, 3, 4, TT_MACRO, TM_INTERPOLATION), // "{{aaa}}" clipped to cols 3..7
            tok(0, 8, 2, TT_MACRO, TM_INTERPOLATION), // "{{bbb}}" clipped to cols 8..10
        ]
    );
    assert_ordered_nonoverlapping(&single);

    // A range crossing the line ending: line-0 tail plus a clipped line-1 head.
    let across = semantic_range(&mut fixture, uri.as_str(), (0, 10), (1, 6));
    assert_eq!(
        across,
        vec![
            tok(0, 10, 5, TT_MACRO, TM_INTERPOLATION), // "{{bbb}}" clipped to cols 10..15
            tok(1, 3, 3, TT_MACRO, TM_INTERPOLATION),  // "{{ccc}}" clipped to cols 3..6
        ]
    );
    assert_ordered_nonoverlapping(&across);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_family_precedence_full_and_range() {
    // Criterion 10: an interpolation inside a directive option value proves F1
    // owns the `{{ }}` bytes and clips the F2 string around it, and the wiki
    // separators exercise structural-subtoken precedence — for both requests.
    let workspace = tempfile::tempdir().unwrap();
    let text = "::file ./z.md when=\"{{ e }}\"\nSee [[doc#H]] here.\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(
        full,
        vec![
            tok(0, 0, 6, TT_MACRO, TM_DIRECTIVE),      // ::file
            tok(0, 7, 6, TT_STRING, TM_DIRECTIVE),     // ./z.md
            tok(0, 14, 4, TT_PROPERTY, TM_DIRECTIVE),  // when
            tok(0, 19, 1, TT_STRING, TM_DIRECTIVE),    // opening quote (F2, clipped)
            tok(0, 20, 7, TT_MACRO, TM_INTERPOLATION), // {{ e }} (F1 owns the interior)
            tok(0, 27, 1, TT_STRING, TM_DIRECTIVE),    // closing quote (F2, clipped)
            tok(1, 4, 2, TT_MACRO, TM_WIKI),           // [[
            tok(1, 6, 3, TT_STRING, TM_WIKI),          // doc
            tok(1, 9, 1, TT_MACRO, TM_WIKI),           // # (structural separator over the frame)
            tok(1, 10, 1, TT_STRING, TM_WIKI),         // H
            tok(1, 11, 2, TT_MACRO, TM_WIKI),          // ]]
        ]
    );
    assert_ordered_nonoverlapping(&full);

    // A range starting inside the interpolation keeps the precedence: F1 owns the
    // interior, the trailing quote stays F2.
    let ranged = semantic_range(&mut fixture, uri.as_str(), (0, 22), (0, 28));
    assert_eq!(
        ranged,
        vec![
            tok(0, 22, 5, TT_MACRO, TM_INTERPOLATION), // "{{ e }}" clipped to cols 22..27
            tok(0, 27, 1, TT_STRING, TM_DIRECTIVE),    // closing quote
        ]
    );
    assert_ordered_nonoverlapping(&ranged);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_master_switch_toggles_and_requests_refresh() {
    // Criterion 7: a runtime `true -> false -> true` master-switch transition
    // suppresses and restores emission without a restart, requesting a refresh
    // each time the capable client honors one.
    let workspace = tempfile::tempdir().unwrap();
    let text = "Body {{ title }} here.\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", true));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let enabled = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(enabled, vec![tok(0, 5, 11, TT_MACRO, TM_INTERPOLATION)]);

    // Disable → refresh requested, then a live request returns an empty stream.
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "semantic_tokens": { "enable": false } } } }),
    );
    assert!(
        fixture.wait_for_server_request("workspace/semanticTokens/refresh"),
        "disabling emission must request a refresh"
    );
    assert!(
        semantic_full(&mut fixture, uri.as_str()).is_empty(),
        "the master switch must suppress emission without a restart"
    );

    // Re-enable → refresh requested again, tokens return identically.
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "semantic_tokens": { "enable": true } } } }),
    );
    assert!(
        fixture.wait_for_server_request("workspace/semanticTokens/refresh"),
        "re-enabling emission must request a refresh"
    );
    assert_eq!(semantic_full(&mut fixture, uri.as_str()), enabled);

    fixture.shutdown();
}

#[test]
fn semantic_tokens_two_configs_issue_distinct_refreshes_routed_through_loop() {
    // Criterion 7, through the real router loop: two token-affecting config
    // changes sent before either refresh is answered must issue two distinct
    // `workspace/semanticTokens/refresh` request ids, and routing a success and
    // an error `Message::Response` back into the `RefreshLedger` must neither
    // terminate nor disturb the session.
    let workspace = tempfile::tempdir().unwrap();
    let text = "Body {{ title }} here.\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", true));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let enabled = semantic_full(&mut fixture, uri.as_str());
    assert_eq!(enabled, vec![tok(0, 5, 11, TT_MACRO, TM_INTERPOLATION)]);

    // Two token-affecting changes back-to-back, before replying to either
    // refresh: disabling then re-enabling the master switch each warrants a
    // refresh, so the ledger must allocate two ids in flight at once.
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "semantic_tokens": { "enable": false } } } }),
    );
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "semantic_tokens": { "enable": true } } } }),
    );

    let first = fixture
        .take_server_request("workspace/semanticTokens/refresh")
        .expect("first refresh request");
    let second = fixture
        .take_server_request("workspace/semanticTokens/refresh")
        .expect("second refresh request");
    assert_ne!(
        first.id, second.id,
        "two in-flight refreshes must carry distinct request ids: {:?} vs {:?}",
        first.id, second.id
    );

    // Answer the first with a success and the second with a client rejection;
    // both route through the loop's `Message::Response` arm. The error must be
    // retired at warn (not fail the loop), the success retired quietly.
    fixture.respond(Response::new_ok(first.id.clone(), Value::Null));
    fixture.respond(Response::new_err(
        second.id.clone(),
        -32803,
        "client rejected refresh".to_string(),
    ));

    // The session stays fully responsive: a normal follow-up request answers,
    // and the net `false -> true` toggle leaves the token stream restored.
    assert_eq!(
        semantic_full(&mut fixture, uri.as_str()),
        enabled,
        "the server must remain responsive after routing both refresh responses"
    );

    fixture.shutdown();
}

#[test]
fn semantic_tokens_config_applies_without_refresh_capability() {
    // Criterion 7: a client that does not honor refresh still sees the new config
    // on every later request, and the server sends no refresh it cannot use.
    let workspace = tempfile::tempdir().unwrap();
    let text = "Body {{ title }} here.\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    assert_eq!(
        semantic_full(&mut fixture, uri.as_str()),
        vec![tok(0, 5, 11, TT_MACRO, TM_INTERPOLATION)]
    );

    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "semantic_tokens": { "enable": false } } } }),
    );
    // The next request (which also pumps any server-initiated message) reflects
    // the new config.
    assert!(
        semantic_full(&mut fixture, uri.as_str()).is_empty(),
        "config must apply to later requests even without refresh support"
    );
    assert!(
        !fixture
            .server_requests
            .iter()
            .any(|request| request.method == "workspace/semanticTokens/refresh"),
        "a refresh must not be sent to a client that does not support it"
    );

    fixture.shutdown();
}

#[test]
fn semantic_tokens_wiki_enable_suppresses_only_f4() {
    // Criterion 7: `wiki.enable = false` drops F4 wiki tokens while leaving F1
    // interpolation tokens untouched.
    let workspace = tempfile::tempdir().unwrap();
    let text = "{{ a }} [[doc]]\n";
    std::fs::write(workspace.path().join("doc.md"), text).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(semantic_capable_params(workspace.path(), "utf-16", false));
    let uri = url::Url::from_file_path(workspace.path().join("doc.md")).unwrap();
    open(&fixture, uri.as_str(), text);

    let full = semantic_full(&mut fixture, uri.as_str());
    assert!(
        full.iter().any(|token| token.modifiers & TM_WIKI != 0),
        "wiki tokens must be present by default: {full:?}"
    );

    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "wiki": { "enable": false } } } }),
    );
    let suppressed = semantic_full(&mut fixture, uri.as_str());
    assert!(
        suppressed.iter().all(|token| token.modifiers & TM_WIKI == 0),
        "wiki.enable = false must suppress every F4 token: {suppressed:?}"
    );
    assert_eq!(
        suppressed,
        vec![tok(0, 0, 7, TT_MACRO, TM_INTERPOLATION)],
        "the F1 interpolation must survive the wiki gate: {suppressed:?}"
    );

    fixture.shutdown();
}
