//! The slow-request log at the dispatch boundary: one completion event per
//! request or notification on every terminal path.
//!
//! Each test drives [`run_server`] over an in-memory connection with a clock
//! that moves a fixed step on every read, and captures the server thread's
//! `tracing` output. Exact threshold boundaries are covered in
//! `request_timing`'s own tests with a manual clock; these assert the wiring.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

use super::*;
use crate::log_capture::CapturedLogs;

/// A request method whose handler panics, so the panic boundary is testable.
pub(super) const PANIC_METHOD: &str = "dmls/test/panic";

const SECRET_FRONTMATTER: &str = "frontmatter-secret-4b1d";
const SECRET_BODY: &str = "body-secret-9e2c";

/// A clock that moves `step` forward on every read.
#[derive(Debug)]
struct SteppingClock {
    now: Mutex<Instant>,
    step: Duration,
}

impl Clock for SteppingClock {
    fn now(&self) -> Instant {
        let mut now = self.now.lock().unwrap();
        *now += self.step;
        *now
    }
}

struct Session {
    _dir: tempfile::TempDir,
    root: PathBuf,
    client: Connection,
    server: std::thread::JoinHandle<()>,
    logs: CapturedLogs,
}

fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs")).unwrap();
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(
        root.join(".git/config"),
        "[core]\n\trepositoryformatversion = 0\n\tbare = false\n",
    )
    .unwrap();
}

fn uri_of(path: &Path) -> String {
    crate::workspace::file_path_to_uri(path).unwrap().as_str().to_string()
}

impl Session {
    /// Starts an initialized session whose clock moves `step` per read.
    fn start(step: Duration) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
        init_repository(&root);
        std::fs::create_dir_all(root.join("home")).unwrap();
        let snapshot = RequestSnapshot::new(&root).with_home(Some(root.join("home")));
        let clock = Arc::new(SteppingClock { now: Mutex::new(Instant::now()), step });
        let options = RunOptions::new(snapshot).with_clock(clock);

        let logs = CapturedLogs::default();
        let dispatch = logs.dispatch();
        let (server_side, client) = Connection::memory();
        let server = std::thread::spawn(move || {
            tracing::dispatcher::with_default(&dispatch, || {
                run_server(server_side, options).expect("server exits cleanly");
            });
        });
        let session = Self { _dir: dir, root, client, server, logs };
        session.send(Message::Request(Request::new(
            RequestId::from(0),
            "initialize".to_string(),
            json!({ "processId": null, "rootUri": uri_of(&session.root), "capabilities": {} }),
        )));
        session.response(RequestId::from(0));
        session.notify("initialized", json!({}));
        session
    }

    fn doc_uri(&self) -> String {
        uri_of(&self.root.join("doc.md"))
    }

    fn send(&self, message: Message) {
        self.client.sender.send(message).unwrap();
    }

    fn notify(&self, method: &str, params: Value) {
        self.send(Message::Notification(Notification::new(method.to_string(), params)));
    }

    fn open_doc(&self) {
        let text = format!("---\ntitle: {SECRET_FRONTMATTER}\n---\n\n# Heading\n\n{SECRET_BODY}\n");
        self.notify(
            "textDocument/didOpen",
            json!({ "textDocument": {
                "uri": self.doc_uri(), "languageId": "markdown", "version": 1, "text": text
            } }),
        );
    }

    fn request(&self, id: i32, method: &str, params: Value) -> Response {
        self.send(Message::Request(Request::new(RequestId::from(id), method.to_string(), params)));
        self.response(RequestId::from(id))
    }

    fn response(&self, id: RequestId) -> Response {
        loop {
            match self.client.receiver.recv_timeout(Duration::from_secs(30)).expect("server answers") {
                Message::Response(response) if response.id == id => return response,
                _ => {}
            }
        }
    }

    fn hover_params(&self) -> Value {
        json!({ "textDocument": { "uri": self.doc_uri() }, "position": { "line": 4, "character": 3 } })
    }

    /// Shuts the server down and returns everything its thread logged.
    fn finish(self) -> CapturedLogs {
        self.send(Message::Request(Request::new(
            RequestId::from(-1),
            "shutdown".to_string(),
            Value::Null,
        )));
        self.response(RequestId::from(-1));
        self.notify("exit", Value::Null);
        self.server.join().expect("server thread");
        self.logs
    }
}

/// The request-completion lines for request `id`.
fn request_completions(logs: &CapturedLogs, id: i32) -> Vec<String> {
    let needle = format!(" id={id} ");
    logs.lines()
        .into_iter()
        .filter(|line| line.contains("request complete") || line.contains("slow request"))
        .filter(|line| line.contains(&needle))
        .collect()
}

#[test]
fn a_fast_hover_logs_one_debug_request_complete_with_its_stages() {
    let session = Session::start(Duration::ZERO);
    session.open_doc();
    let uri = session.doc_uri();
    let response = session.request(7, "textDocument/hover", session.hover_params());
    assert!(response.error.is_none(), "{response:?}");
    let logs = session.finish();

    let lines = request_completions(&logs, 7);
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.starts_with("DEBUG dmls::request_timing: request complete"), "{line}");
    for field in [
        "method=\"textDocument/hover\"".to_string(),
        format!("uri=\"{uri}\""),
        "outcome=\"ok\"".to_string(),
        "elapsed_ms=0".to_string(),
        "overlay_build_ms=0".to_string(),
        "schema_assembly_ms=0".to_string(),
        "trigger_registry_ms=0".to_string(),
        "provider_chain_ms=0".to_string(),
        "waited_for_file_list=false".to_string(),
        "waited_for_index=false".to_string(),
    ] {
        assert!(line.contains(&field), "missing {field}: {line}");
    }
    assert!(!line.contains("cancelled"), "{line}");
}

#[test]
fn a_slow_hover_logs_one_warn_slow_request() {
    let session = Session::start(Duration::from_secs(1));
    session.open_doc();
    session.request(7, "textDocument/hover", session.hover_params());
    let logs = session.finish();

    let lines = request_completions(&logs, 7);
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.starts_with(" WARN dmls::request_timing: slow request"), "{line}");
    assert!(line.contains("method=\"textDocument/hover\""), "{line}");
    // Every stage is a positive elapsed figure under a clock that always moves.
    for stage in ["overlay_build_ms=", "schema_assembly_ms=", "trigger_registry_ms=", "provider_chain_ms="] {
        assert!(line.contains(stage), "missing {stage}: {line}");
        assert!(!line.contains(&format!("{stage}0 ")), "{stage} did not move: {line}");
    }
}

#[test]
fn a_workspace_symbol_request_omits_uri_and_overlay_stages() {
    let session = Session::start(Duration::ZERO);
    let response = session.request(4, "workspace/symbol", json!({ "query": "Heading" }));
    assert!(response.error.is_none(), "{response:?}");
    let logs = session.finish();

    let lines = request_completions(&logs, 4);
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.contains("method=\"workspace/symbol\""), "{line}");
    assert!(!line.contains("uri="), "{line}");
    assert!(line.contains("provider_chain_ms=0"), "{line}");
    assert!(!line.contains("overlay_build_ms"), "{line}");
}

#[test]
fn error_responses_emit_one_error_completion() {
    let session = Session::start(Duration::ZERO);
    let unknown = session.request(5, "dmls/no-such-method", json!({}));
    assert!(unknown.error.is_some());
    let malformed = session.request(6, "textDocument/hover", json!({ "textDocument": 3 }));
    assert!(malformed.error.is_some());
    let logs = session.finish();

    for id in [5, 6] {
        let lines = request_completions(&logs, id);
        assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
        assert!(lines[0].contains("outcome=\"error\""), "{}", lines[0]);
    }
}

#[test]
fn a_caught_panic_emits_one_panic_completion_and_dispatch_continues() {
    let session = Session::start(Duration::ZERO);
    let panicked = session.request(8, PANIC_METHOD, json!({}));
    assert_eq!(panicked.error.unwrap().code, ErrorCode::InternalError as i32);
    let after = session.request(9, "workspace/symbol", json!({ "query": "" }));
    assert!(after.error.is_none());
    let logs = session.finish();

    let lines = request_completions(&logs, 8);
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    assert!(lines[0].contains("outcome=\"panic\""), "{}", lines[0]);
    assert_eq!(request_completions(&logs, 9).len(), 1);
}

#[test]
fn a_cancelled_request_emits_one_cancelled_completion() {
    let session = Session::start(Duration::ZERO);
    session.notify("$/cancelRequest", json!({ "id": 11 }));
    let response = session.request(11, "textDocument/hover", session.hover_params());
    assert_eq!(response.error.unwrap().code, ErrorCode::RequestCanceled as i32);
    let logs = session.finish();

    let lines = request_completions(&logs, 11);
    assert_eq!(lines.len(), 1, "{:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.contains("cancelled=true"), "{line}");
    assert!(line.contains("outcome=\"cancelled\""), "{line}");
    assert!(line.contains("elapsed_ms=0"), "{line}");
    // The handler never ran.
    assert!(!line.contains("provider_chain_ms"), "{line}");
}

#[test]
fn notifications_log_their_own_completion_without_response_timing() {
    let session = Session::start(Duration::from_secs(1));
    session.open_doc();
    session.notify("$/cancelRequest", json!({ "id": 999 }));
    let uri = session.doc_uri();
    let logs = session.finish();

    let opens = logs.matching("method=\"textDocument/didOpen\"");
    let completions: Vec<_> = opens
        .iter()
        .filter(|line| line.contains("slow notification") || line.contains("notification complete"))
        .collect();
    assert_eq!(completions.len(), 1, "{:#?}", logs.lines());
    let line = completions[0];
    assert!(line.starts_with(" WARN dmls::request_timing: slow notification"), "{line}");
    assert!(line.contains(&format!("uri=\"{uri}\"")), "{line}");
    assert!(!line.contains(" id="), "{line}");
    assert!(!line.contains("request complete") && !line.contains("slow request"), "{line}");
    // A notification that names no document omits `uri`.
    let cancels: Vec<_> = logs
        .matching("method=\"$/cancelRequest\"")
        .into_iter()
        .filter(|line| line.contains("slow notification"))
        .collect();
    assert_eq!(cancels.len(), 1, "{:#?}", logs.lines());
    assert!(!cancels[0].contains("uri="), "{}", cancels[0]);
}

#[test]
fn no_source_text_or_frontmatter_value_is_logged() {
    let session = Session::start(Duration::from_secs(1));
    session.open_doc();
    session.request(7, "textDocument/hover", session.hover_params());
    session.request(8, "textDocument/documentSymbol", json!({ "textDocument": { "uri": session.doc_uri() } }));
    let logs = session.finish();

    let lines = logs.lines();
    assert!(lines.iter().any(|line| line.contains("slow request")), "{lines:#?}");
    for secret in [SECRET_FRONTMATTER, SECRET_BODY, "# Heading"] {
        let leaked: Vec<_> = lines.iter().filter(|line| line.contains(secret)).collect();
        assert!(leaked.is_empty(), "{secret} was logged: {leaked:#?}");
    }
}

/// The incident: with the registry up, an unrelated schema-root file goes
/// mid-edit malformed and every later request used to re-warn. Now it logs
/// nothing, and a claimed envelope breaking warns once however many requests
/// follow.
#[test]
fn a_mid_edit_unrelated_schema_file_logs_nothing_and_a_broken_trigger_warns_once() {
    let session = Session::start(Duration::ZERO);
    let schemas = session.root.join("schemas");
    std::fs::create_dir_all(&schemas).unwrap();
    let envelope = schemas.join("title.trigger.yaml");
    std::fs::write(&envelope, "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema: title.yaml\n").unwrap();
    std::fs::write(schemas.join("title.yaml"), "$schema:\n  model: string\n").unwrap();
    let changed = |path: &Path, kind: u8| {
        session.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [{ "uri": uri_of(path), "type": kind }] }),
        );
    };
    session.open_doc();
    session.request(1, "textDocument/hover", session.hover_params());

    let incident = schemas.join("feature-review.yaml");
    std::fs::write(&incident, include_str!("../../../lib/tests/fixtures/triggers/feature-review-incident.yaml")).unwrap();
    changed(&incident, 1);
    for id in 2..7 {
        assert!(session.request(id, "textDocument/hover", session.hover_params()).error.is_none());
    }

    std::fs::write(&envelope, "kind: trigger-schema\nmatch: {}\n").unwrap();
    changed(&envelope, 2);
    for id in 7..12 {
        assert!(session.request(id, "textDocument/hover", session.hover_params()).error.is_none());
    }
    let logs = session.finish();

    let registry_lines = logs.matching("trigger registry");
    assert!(
        registry_lines.iter().all(|line| !line.contains("feature-review.yaml")),
        "the unrelated file is never a trigger failure: {registry_lines:#?}"
    );
    let warnings: Vec<&String> = registry_lines.iter().filter(|line| line.starts_with(" WARN")).collect();
    assert_eq!(warnings.len(), 1, "{registry_lines:#?}");
    assert!(warnings[0].contains("retaining last-good trigger registry"), "{}", warnings[0]);
    assert!(warnings[0].contains(&format!("trigger_file={}", envelope.display())), "{}", warnings[0]);
}
