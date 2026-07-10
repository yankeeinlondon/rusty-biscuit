//! Ignored end-to-end red scaffolds for `suggest(...)` diagnostics/completion.
//!
//! These tests use the same bounded in-memory LSP transport as
//! `lsp_session.rs`. They are selected by the package's `level2_` filter but
//! remain ignored until their implementation phases land.

use std::{sync::mpsc, time::Duration};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

const INLINE: &str = include_str!("fixtures/suggest_constraint/inline.md");
const PURE: &str = include_str!("fixtures/suggest_constraint/pure.yaml");
const TAGGED: &str = include_str!("fixtures/suggest_constraint/tagged.yaml");
const COMPLETION: &str = include_str!("fixtures/suggest_constraint/completion.md");
const UNIONS: &str = include_str!("fixtures/suggest_constraint/unions.md");
const RAW_SCHEMA: &str = include_str!("fixtures/suggest_constraint/raw-schema.json");
const RAW_CONSUMER: &str = include_str!("fixtures/suggest_constraint/raw-consumer.md");

struct ClientFixture {
    client: Connection,
    server_outcome: mpsc::Receiver<Result<(), String>>,
    next_id: i32,
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

    fn initialize(&mut self, root: &std::path::Path) {
        let root_uri = url::Url::from_directory_path(root).unwrap();
        let response = self.request(
            "initialize",
            json!({
                "processId": null,
                "clientInfo": { "name": "Neovim", "version": "0.11.0" },
                "capabilities": {
                    "general": { "positionEncodings": ["utf-8", "utf-16"] },
                    "workspace": { "configuration": true }
                },
                "workspaceFolders": [
                    { "uri": root_uri.as_str(), "name": "suggestions" }
                ]
            }),
        );
        assert!(response.error.is_none(), "initialize failed: {:?}", response.error);
        self.notify("initialized", json!({}));
    }

    fn request(&mut self, method: &str, params: Value) -> Response {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.client
            .sender
            .send(Message::Request(Request::new(id.clone(), method.to_string(), params)))
            .expect("send request");
        loop {
            match self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("response before timeout")
            {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(_) => {}
                other => panic!("unexpected message while waiting for response: {other:?}"),
            }
        }
    }

    fn notify(&self, method: &str, params: Value) {
        self.client
            .sender
            .send(Message::Notification(Notification::new(method.to_string(), params)))
            .expect("send notification");
    }

    fn diagnostics(&mut self, uri: &str) -> Vec<Value> {
        loop {
            if let Some(position) = self.notifications.iter().rposition(|notification| {
                notification.method == "textDocument/publishDiagnostics"
                    && notification.params["uri"] == json!(uri)
            }) {
                return self.notifications.remove(position).params["diagnostics"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default();
            }
            match self
                .client
                .receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("diagnostics before timeout")
            {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(_) => {}
                other => panic!("unexpected message while waiting for diagnostics: {other:?}"),
            }
        }
    }

    fn completion(&mut self, uri: &str, line: u32, character: u32) -> Vec<Value> {
        self.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }),
        )
        .result
        .expect("completion response")
        .as_array()
        .cloned()
        .expect("completion array")
    }

    fn shutdown(self) {
        let mut fixture = self;
        let response = fixture.request("shutdown", Value::Null);
        assert!(response.error.is_none(), "shutdown failed: {:?}", response.error);
        fixture.notify("exit", Value::Null);
        assert_eq!(
            fixture
                .server_outcome
                .recv_timeout(Duration::from_secs(10))
                .expect("server thread finished"),
            Ok(())
        );
    }
}

fn open(fixture: &ClientFixture, uri: &str, language_id: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text
            }
        }),
    );
}

fn assert_invalid_suggestion(diagnostic: &Value, line: u32, start: u32, end: u32) {
    assert_eq!(diagnostic["severity"], json!(2));
    assert_eq!(diagnostic["source"], json!("darkmatter.schema"));
    assert_eq!(diagnostic["code"], json!("dm.schema.invalid_suggestion"));
    assert_eq!(
        diagnostic["range"],
        json!({
            "start": { "line": line, "character": start },
            "end": { "line": line, "character": end }
        })
    );
}

fn labels(items: &[Value]) -> Vec<&str> {
    items.iter().filter_map(|item| item["label"].as_str()).collect()
}

#[test]
#[ignore = "red acceptance scaffold; enable in suggestion diagnostics phase"]
fn level2_suggest_phase1_inline_warning_has_exact_argument_range() {
    let workspace = tempfile::tempdir().unwrap();
    let path = workspace.path().join("inline.md");
    std::fs::write(&path, INLINE).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(workspace.path());
    open(&fixture, uri.as_str(), "markdown", INLINE);
    let diagnostics = fixture.diagnostics(uri.as_str());
    let suggestion = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("invalid suggestion diagnostic");
    assert_invalid_suggestion(suggestion, 2, 35, 39);
    fixture.shutdown();
}

#[test]
#[ignore = "red acceptance scaffold; enable in standalone schema phase"]
fn level2_suggest_phase1_standalone_ranges_and_completion() {
    let workspace = tempfile::tempdir().unwrap();
    let pure_path = workspace.path().join("pure.yaml");
    let tagged_path = workspace.path().join("tagged.yaml");
    std::fs::write(&pure_path, PURE).unwrap();
    std::fs::write(&tagged_path, TAGGED).unwrap();
    let pure_uri = url::Url::from_file_path(&pure_path).unwrap();
    let tagged_uri = url::Url::from_file_path(&tagged_path).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(workspace.path());
    open(&fixture, pure_uri.as_str(), "yaml", PURE);
    open(&fixture, tagged_uri.as_str(), "yaml", TAGGED);

    let pure_diagnostics = fixture.diagnostics(pure_uri.as_str());
    let pure_suggestion = pure_diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("pure-envelope warning");
    assert_invalid_suggestion(pure_suggestion, 2, 35, 39);

    let tagged_diagnostics = fixture.diagnostics(tagged_uri.as_str());
    let tagged_suggestion = tagged_diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == json!("dm.schema.invalid_suggestion"))
        .expect("tagged-envelope warning");
    assert_invalid_suggestion(tagged_suggestion, 3, 35, 39);
    fixture.shutdown();
}

#[test]
#[ignore = "red acceptance scaffold; enable in suggestion completion phase"]
fn level2_suggest_phase1_completion_positions() {
    let workspace = tempfile::tempdir().unwrap();
    let path = workspace.path().join("completion.md");
    std::fs::write(&path, COMPLETION).unwrap();
    let uri = url::Url::from_file_path(path).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(workspace.path());
    open(&fixture, uri.as_str(), "markdown", COMPLETION);

    let scalar = fixture.completion(uri.as_str(), 6, 9);
    assert_eq!(labels(&scalar), vec!["green"]);
    assert_eq!(scalar[0]["textEdit"]["newText"], json!("\"green\""));

    let nested = fixture.completion(uri.as_str(), 8, 10);
    assert_eq!(labels(&nested), vec!["slow"]);
    assert_eq!(nested[0]["textEdit"]["newText"], json!("\"slow\""));

    let block_array = fixture.completion(uri.as_str(), 10, 6);
    assert_eq!(labels(&block_array), vec!["alpha"]);
    assert_eq!(block_array[0]["textEdit"]["newText"], json!("\"alpha\""));

    let flow_array = fixture.completion(uri.as_str(), 11, 12);
    assert_eq!(labels(&flow_array), vec!["0.25", "0.5", "1"]);
    assert_eq!(flow_array[0]["textEdit"]["newText"], json!("0.25"));
    fixture.shutdown();
}

#[test]
#[ignore = "red acceptance scaffold; enable in suggestion completion phase"]
fn level2_suggest_phase1_union_selection_and_raw_schema_exclusion() {
    let workspace = tempfile::tempdir().unwrap();
    let union_path = workspace.path().join("unions.md");
    let raw_path = workspace.path().join("raw-schema.json");
    let consumer_path = workspace.path().join("raw-consumer.md");
    std::fs::write(&union_path, UNIONS).unwrap();
    std::fs::write(&raw_path, RAW_SCHEMA).unwrap();
    std::fs::write(&consumer_path, RAW_CONSUMER).unwrap();
    let union_uri = url::Url::from_file_path(union_path).unwrap();
    let consumer_uri = url::Url::from_file_path(consumer_path).unwrap();

    let mut fixture = ClientFixture::start();
    fixture.initialize(workspace.path());
    open(&fixture, union_uri.as_str(), "markdown", UNIONS);
    open(&fixture, consumer_uri.as_str(), "markdown", RAW_CONSUMER);

    let property_union = fixture.completion(union_uri.as_str(), 8, 10);
    assert_eq!(labels(&property_union), vec!["second"]);
    let root_union = fixture.completion(union_uri.as_str(), 9, 8);
    assert_eq!(labels(&root_union), vec!["arm-one"]);
    assert!(
        fixture.completion(consumer_uri.as_str(), 2, 9).is_empty(),
        "raw JSON Schema annotations must not activate suggestion completion"
    );
    fixture.shutdown();
}
