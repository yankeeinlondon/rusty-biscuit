//! Spec acceptance criterion 7: the language server is **passive**.
//!
//! Driving every read-side request across a document dense with `::shell`,
//! `$(...)`, `predict_conflicts(...)`, and remote-URL constructs must analyze
//! them (a dangerous command is diagnosed) while running **nothing**: no child
//! process is spawned (a sentinel a shell directive would create never appears),
//! no Git merge is simulated, and no network fetch is made (the remote
//! constructs resolve instantly, never hanging on I/O).
//!
//! In-memory session (no real terminal or network resource), so it runs in the
//! standard `just test` gate with no terminal harness.

use std::sync::mpsc;
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

struct Fixture {
    client: Connection,
    outcome: mpsc::Receiver<Result<(), String>>,
    next_id: i32,
    notifications: Vec<Notification>,
}

impl Fixture {
    fn start() -> Self {
        let (server_side, client_side) = Connection::memory();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = dmls::run_server(server_side, dmls::RunOptions::default())
                .map_err(|error| error.to_string());
            let _ = tx.send(result);
        });
        Self {
            client: client_side,
            outcome: rx,
            next_id: 0,
            notifications: Vec::new(),
        }
    }

    fn request(&mut self, method: &str, params: Value) -> Response {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.client
            .sender
            .send(Message::Request(Request::new(id.clone(), method.to_string(), params)))
            .expect("send request");
        loop {
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("response before timeout");
            match message {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(_) => {}
                other => panic!("unexpected message: {other:?}"),
            }
        }
    }

    fn notify(&self, method: &str, params: Value) {
        self.client
            .sender
            .send(Message::Notification(Notification::new(method.to_string(), params)))
            .expect("send notification");
    }

    fn wait_for_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        for _ in 0..64 {
            if let Some(position) = self.notifications.iter().rposition(|notification| {
                notification.method == "textDocument/publishDiagnostics"
                    && notification.params["uri"] == json!(uri)
            }) {
                let notification = self.notifications.remove(position);
                return notification.params["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
            }
            let message = self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("diagnostics before timeout");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(_) => {}
                other => panic!("unexpected message: {other:?}"),
            }
        }
        panic!("no diagnostics for {uri}");
    }

    fn initialize(&mut self, root: &std::path::Path) {
        let root_uri = url::Url::from_directory_path(root).unwrap();
        let response = self.request(
            "initialize",
            json!({
                "processId": null,
                "capabilities": {
                    "general": { "positionEncodings": ["utf-8", "utf-16"] },
                    "textDocument": { "foldingRange": { "lineFoldingOnly": true } }
                },
                "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "scratch" } ]
            }),
        );
        assert!(response.error.is_none(), "initialize failed: {:?}", response.error);
        self.notify("initialized", json!({}));
    }

    fn shutdown(mut self) {
        let response = self.request("shutdown", Value::Null);
        assert!(response.error.is_none());
        self.notify("exit", Value::Null);
        let outcome = self.outcome.recv_timeout(Duration::from_secs(10)).expect("server finished");
        assert_eq!(outcome, Ok(()), "server exited with error");
    }
}

#[test]
fn dsl_requests_spawn_no_processes_and_open_no_sockets() {
    let workspace = tempfile::tempdir().unwrap();
    // A sentinel a shell directive *would* create if DMLS ever executed it. Its
    // continued absence after every request is the "no child process" proof.
    let sentinel = workspace.path().join("SENTINEL_SHOULD_NOT_EXIST");
    let sentinel_display = sentinel.to_string_lossy().replace('\\', "/");

    // Frontmatter `$()` + inline `$schema` Expression/meta-schema-typed values + body
    // `::shell` (dangerous + sentinel), a Git conflict prediction, a remote
    // `::url`, and a remote image link — every construct that could touch a
    // process, repository, or socket if the analyzer were not passive. The
    // Expression values exercise frontmatter completion/hover/diagnostics,
    // while the semantic schema values carry nonexistent imports and a remote
    // declaration. All parse, diagnose, or complete without resolving.
    let text = format!(
        "---\ntitle: Passive\ncommand: $(echo pwned)\n$schema:\n  when: expression\n  conflicts: expression\n  definition: type-definition\n  declaration: schema\ndefinition: 'Imported@./missing-types.yaml'\ndeclaration: https://example.com/schema.yaml\nwhen: length(title)\nconflicts: predict_conflicts(\"feature/example\")\n---\n\n\
         # Heading\n\n\
         ::shell rm -rf /tmp/dmls-should-not-run\n\n\
         ::shell touch {sentinel_display}\n\n\
         ::url https://example.com/remote.md\n\n\
         ::file ./missing.md\n\n\
         Conflicts: {{{{ predict_conflicts(\"feature/example\") }}}}\n\n\
         See {{{{ title }}}} and remote ![img](https://example.com/x.png).\n"
    );

    let doc_path = workspace.path().join("doc.md");
    std::fs::write(&doc_path, &text).unwrap();
    let doc_uri = url::Url::from_file_path(&doc_path).unwrap();

    let mut fixture = Fixture::start();
    fixture.initialize(workspace.path());
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": doc_uri.as_str(),
                "languageId": "markdown",
                "version": 1,
                "text": text
            }
        }),
    );

    // Diagnostics analyze the dangerous command without running it.
    let diagnostics = fixture.wait_for_diagnostics(doc_uri.as_str());
    let has_security = diagnostics
        .iter()
        .any(|diagnostic| diagnostic["code"] == json!("dm.security.disallowed_command"));
    assert!(has_security, "the dangerous ::shell must be diagnosed: {diagnostics:?}");

    // Drive every read-side request across the shell/remote spans and the
    // Expression-typed frontmatter values. Each must return promptly (no Git or
    // network hang) and never execute anything. `(11, 15)` lands inside the Git
    // function and `(24, 18)` lands inside the body invocation.
    let positions = [
        (16u32, 8u32),
        (18, 8),
        (20, 8),
        (22, 8),
        (24, 18),
        (26, 8),
        (2, 12),
        (8, 18),
        (9, 18),
        (10, 8),
        (11, 15),
    ];
    for (line, character) in positions {
        for method in [
            "textDocument/hover",
            "textDocument/definition",
            "textDocument/references",
            "textDocument/completion",
        ] {
            let mut params = json!({
                "textDocument": { "uri": doc_uri.as_str() },
                "position": { "line": line, "character": character }
            });
            if method == "textDocument/references" {
                params["context"] = json!({ "includeDeclaration": true });
            }
            let response = fixture.request(method, params);
            assert!(response.error.is_none(), "{method} errored: {:?}", response.error);
        }
    }
    let folding = fixture.request(
        "textDocument/foldingRange",
        json!({ "textDocument": { "uri": doc_uri.as_str() } }),
    );
    assert!(folding.error.is_none());
    let links = fixture.request(
        "textDocument/documentLink",
        json!({ "textDocument": { "uri": doc_uri.as_str() } }),
    );
    assert!(links.error.is_none());

    // Semantic tokens tokenize the `{{ }}`, `::shell`, `::url`, and image spans
    // without executing or fetching anything (spec criterion 8).
    let full = fixture.request(
        "textDocument/semanticTokens/full",
        json!({ "textDocument": { "uri": doc_uri.as_str() } }),
    );
    assert!(full.error.is_none(), "semanticTokens/full errored: {:?}", full.error);
    let range = fixture.request(
        "textDocument/semanticTokens/range",
        json!({
            "textDocument": { "uri": doc_uri.as_str() },
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 28, "character": 0 }
            }
        }),
    );
    assert!(range.error.is_none(), "semanticTokens/range errored: {:?}", range.error);

    // The standalone content-activation path must be equally passive. Neither
    // the named import nor the example reference is opened on a keystroke.
    let standalone = "$schema:\n  payload: 'Imported@./missing-types.yaml'\n  sample: 'string(example(./missing-example.md))'\n";
    let standalone_path = workspace.path().join("standalone.yaml");
    std::fs::write(&standalone_path, standalone).unwrap();
    let standalone_uri = url::Url::from_file_path(&standalone_path).unwrap();
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": standalone_uri.as_str(),
                "languageId": "yaml",
                "version": 1,
                "text": standalone
            }
        }),
    );
    let _ = fixture.wait_for_diagnostics(standalone_uri.as_str());
    for method in ["textDocument/hover", "textDocument/completion"] {
        let response = fixture.request(
            method,
            json!({
                "textDocument": { "uri": standalone_uri.as_str() },
                "position": { "line": 1, "character": 16 }
            }),
        );
        assert!(response.error.is_none(), "{method} errored: {:?}", response.error);
    }

    fixture.shutdown();

    // The marquee guarantee: no shell directive ran, so the sentinel was never
    // created. (A regression that executed directives would materialize it.)
    assert!(
        !sentinel.exists(),
        "a shell directive was executed — the language server is not passive"
    );
}
