//! `textDocument/formatting` over a full in-memory LSP conversation: the
//! library clean transaction runs on the unsaved buffer, returns one
//! whole-document edit only when it changed something, and every refusal or
//! invalid document answers `[]` with exactly one `window/showMessage`
//! Warning whose position is in the negotiated encoding. Configuration
//! findings are shown once per configuration change, never per request.
//!
//! The shared `LspFixture` keeps non-diagnostic notifications to itself, so
//! this module drives its own session, recording every `window/showMessage`.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use darkmatter::markdown::cleanup::rules::MarkdownRuleOverrides;
use darkmatter::markdown::cleanup::test_hooks::install_broken_pass;
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, clean_document};
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::schemas::CleanSchemaConfig;
use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use serde_json::{Value, json};

/// Deadline for one protocol message and for the server's exit.
const BOUND: Duration = Duration::from_secs(10);

/// `window/showMessage`'s `MessageType` values.
const ERROR: u64 = 1;
const WARNING: u64 = 2;

/// One in-memory DMLS session over a workspace directory.
struct Session {
    client: Option<Connection>,
    outcome: mpsc::Receiver<Result<(), String>>,
    server: Option<std::thread::JoinHandle<()>>,
    next_id: i32,
    notifications: Vec<Notification>,
    /// The `initialize` result's server capabilities.
    capabilities: Value,
}

impl Session {
    /// Starts and initializes a server for `root`, offering `encodings`
    /// (an empty list offers none, which negotiates UTF-16).
    fn start(root: &Path, encodings: &[&str]) -> Self {
        let (server_side, client_side) = Connection::memory();
        let (outcome_tx, outcome) = mpsc::channel();
        let snapshot = RequestSnapshot::new(root).with_home(Some(root.join("home")));
        let server = std::thread::spawn(move || {
            let result = dmls::run_server(server_side, dmls::RunOptions::new(snapshot))
                .map_err(|error| error.to_string());
            let _ = outcome_tx.send(result);
        });
        let mut session = Self {
            client: Some(client_side),
            outcome,
            server: Some(server),
            next_id: 0,
            notifications: Vec::new(),
            capabilities: Value::Null,
        };
        let root_uri = url::Url::from_directory_path(root).expect("directory url");
        let general = if encodings.is_empty() {
            json!({})
        } else {
            json!({ "positionEncodings": encodings })
        };
        let response = session.request(
            "initialize",
            json!({
                "processId": null,
                "capabilities": { "general": general },
                "workspaceFolders": [{ "uri": root_uri.as_str(), "name": "scratch" }]
            }),
        );
        assert!(response.error.is_none(), "initialize failed: {:?}", response.error);
        session.capabilities = response.result.expect("initialize result")["capabilities"].clone();
        session.notify("initialized", json!({}));
        session
    }

    fn client(&self) -> &Connection {
        self.client.as_ref().expect("client connection open")
    }

    fn request(&mut self, method: &str, params: Value) -> Response {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.client()
            .sender
            .send(Message::Request(Request::new(id.clone(), method.to_string(), params)))
            .expect("send request");
        loop {
            match self.client().receiver.recv_timeout(BOUND) {
                Ok(Message::Response(response)) if response.id == id => return response,
                Ok(Message::Notification(notification)) => self.notifications.push(notification),
                // Watcher registration and progress requests are
                // fire-and-forget.
                Ok(_) => {}
                Err(_) => panic!("no response to {method} before timeout"),
            }
        }
    }

    fn notify(&self, method: &str, params: Value) {
        self.client()
            .sender
            .send(Message::Notification(Notification::new(method.to_string(), params)))
            .expect("send notification");
    }

    /// Answers once every earlier message has been handled: the router is a
    /// single FIFO loop, so anything sent before this response is buffered.
    fn flush(&mut self) {
        let response = self.request("workspace/symbol", json!({ "query": "" }));
        assert!(response.error.is_none(), "flush failed: {:?}", response.error);
    }

    /// Removes and returns every `window/showMessage` seen so far as
    /// `(type, message)`.
    fn take_messages(&mut self) -> Vec<(u64, String)> {
        let (messages, rest) = std::mem::take(&mut self.notifications)
            .into_iter()
            .partition::<Vec<_>, _>(|notification| notification.method == "window/showMessage");
        self.notifications = rest;
        messages
            .into_iter()
            .map(|notification| {
                (
                    notification.params["type"].as_u64().expect("message type"),
                    notification.params["message"].as_str().expect("message text").to_string(),
                )
            })
            .collect()
    }

    fn open(&self, uri: &str, text: &str) {
        self.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
            }),
        );
    }

    fn change(&self, uri: &str, version: i32, text: &str) {
        self.notify(
            "textDocument/didChange",
            json!({
                "textDocument": { "uri": uri, "version": version },
                "contentChanges": [{ "text": text }]
            }),
        );
    }

    fn configure(&self, settings: Value) {
        self.notify("workspace/didChangeConfiguration", json!({ "settings": settings }));
    }

    /// The edits a formatting request returns.
    fn format(&mut self, uri: &str) -> Vec<Value> {
        let response = self.request(
            "textDocument/formatting",
            json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": 2, "insertSpaces": true }
            }),
        );
        assert!(response.error.is_none(), "formatting failed: {:?}", response.error);
        response.result.expect("formatting result").as_array().expect("edit array").clone()
    }

    fn shutdown(mut self) {
        let response = self.request("shutdown", Value::Null);
        assert!(response.error.is_none(), "shutdown failed: {:?}", response.error);
        let outcome = self.release();
        assert_eq!(outcome, Some(Ok(())), "the server exits cleanly after shutdown");
    }

    /// Sends `exit`, closes the client end, and waits (bounded) for the
    /// server's outcome; a server still running at the bound is detached
    /// rather than joined forever.
    fn release(&mut self) -> Option<Result<(), String>> {
        let server = self.server.take()?;
        if let Some(client) = self.client.take() {
            let _ = client
                .sender
                .send(Message::Notification(Notification::new("exit".to_string(), Value::Null)));
        }
        let outcome = self.outcome.recv_timeout(BOUND).ok();
        if outcome.is_some() {
            let _ = server.join();
        }
        outcome
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.release();
    }
}

fn file_uri(path: &Path) -> String {
    url::Url::from_file_path(path).expect("file url").to_string()
}

/// The one edit's text, asserting it replaces the whole of `source`.
#[track_caller]
fn whole_document_edit(edits: &[Value], source: &str) -> String {
    assert_eq!(edits.len(), 1, "one whole-document edit: {edits:?}");
    let lines = source.split('\n').count() - 1;
    let last_line = source.rsplit('\n').next().unwrap_or_default();
    assert_eq!(edits[0]["range"]["start"], json!({ "line": 0, "character": 0 }));
    assert_eq!(
        edits[0]["range"]["end"],
        json!({ "line": lines, "character": last_line.encode_utf16().count() })
    );
    edits[0]["newText"].as_str().expect("new text").to_string()
}

/// Exactly one Warning, returned for message assertions.
#[track_caller]
fn one_warning(session: &mut Session) -> String {
    let messages = session.take_messages();
    assert_eq!(messages.len(), 1, "exactly one message: {messages:?}");
    assert_eq!(messages[0].0, WARNING, "{messages:?}");
    messages[0].1.clone()
}

/// What `clean_document` itself returns for `source` with default rules and
/// no file-backed schema discovery.
fn library_clean(source: &str, root: &Path) -> (CleanOutcome, String) {
    let result = clean_document(
        &CleanRequest {
            source,
            overrides: &MarkdownRuleOverrides::default(),
            workspace: Some(&MarkdownRuleOverrides::default()),
            document_path: None,
        },
        || {
            Ok::<_, std::convert::Infallible>(CleanSchemaConfig::new(
                biscuit_file::FileResolutionContext::new(root),
            ))
        },
    )
    .expect("clean_document runs");
    (result.outcome, result.text)
}

#[test]
fn formatting_reads_the_unsaved_buffer_not_the_file() {
    let root = tempfile::tempdir().expect("tempdir");
    let path = root.path().join("doc.md");
    std::fs::write(&path, "# Saved\n\nAlready clean.\n").expect("write");
    let uri = file_uri(&path);
    let mut session = Session::start(root.path(), &[]);

    session.open(&uri, "# Saved\n\nAlready clean.\n");
    let buffer = "1. a\n3. b\n8. c\n";
    session.change(&uri, 2, buffer);
    let edits = session.format(&uri);

    assert_eq!(whole_document_edit(&edits, buffer), "1. a\n2. b\n3. c\n");
    assert!(session.take_messages().is_empty());
    session.shutdown();
}

#[test]
fn an_unchanged_document_returns_no_edits_and_no_message() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);

    // Soft breaks, a `1,1,1` list, and CRLF terminators are all kept.
    session.open(&uri, "# Title\r\n\r\nOne line\r\nwrapped.\r\n\r\n1. a\r\n1. b\r\n1. c\r\n");
    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    assert!(session.take_messages().is_empty());
    session.shutdown();
}

/// A refusal injected after the rules ran: the link destination changes.
fn retarget_link(candidate: &str) -> String {
    candidate.replacen("(/one)", "(/two)", 1)
}

/// The refused document: on line 3, `é` (2 bytes, 1 UTF-16 unit) and `🙂`
/// (4 bytes, 2 units) precede the link the injected pass corrupts, so the
/// link is at byte column 12 and UTF-16 column 9 (1-based).
const NON_ASCII_REFUSAL: &str = "# Title\n\né🙂 see [a](/one).\n";

fn injected_refusal_warning(encodings: &[&str]) -> String {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), encodings);
    session.open(&uri, NON_ASCII_REFUSAL);

    let _broken = install_broken_pass(retarget_link);
    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    let warning = one_warning(&mut session);
    session.shutdown();
    warning
}

#[test]
fn an_injected_refusal_reports_a_utf16_column_by_default() {
    let warning = injected_refusal_warning(&[]);

    assert!(warning.contains("cleanup left the document unchanged"), "{warning}");
    assert!(warning.contains("at line 3, column 9:"), "{warning}");
}

#[test]
fn an_injected_refusal_reports_a_utf8_column_when_negotiated() {
    let warning = injected_refusal_warning(&["utf-8"]);

    assert!(warning.contains("cleanup left the document unchanged"), "{warning}");
    assert!(warning.contains("at line 3, column 12:"), "{warning}");
}

#[test]
fn an_injected_refusal_on_an_ascii_document_names_the_block() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    // The unlabeled fence would be changed, so the refusal discards a real
    // candidate.
    session.open(&uri, "# Title\n\nSee [a](/one).\n\n```\ncode\n```\n");

    let _broken = install_broken_pass(retarget_link);
    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    let warning = one_warning(&mut session);
    assert!(warning.contains("cleanup left the document unchanged"), "{warning}");
    assert!(warning.contains("at line 3, column 5:"), "{warning}");
    session.shutdown();
}

#[test]
fn an_unpaired_block_directive_declines_with_one_warning() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    session.open(&uri, "# Title\n\n::block when\n\n1. a\n3. b\n");

    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    let warning = one_warning(&mut session);
    assert!(warning.contains("cleanup left the document unchanged"), "{warning}");
    assert!(warning.contains("malformed directive region"), "{warning}");
    assert!(warning.contains("at line 3, column 1:"), "{warning}");
    session.shutdown();
}

#[test]
fn unrepairable_frontmatter_returns_no_edits_and_one_warning() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    session.open(&uri, "---\ntitle: a\ntitle: b\n---\n\n```\ncode\n```\n");

    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    let warning = one_warning(&mut session);
    assert!(warning.contains("frontmatter is not valid YAML at line"), "{warning}");
    session.shutdown();
}

#[test]
fn repairable_frontmatter_is_repaired_as_the_library_repairs_it() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    // `@` is a reserved indicator, so the authored YAML does not parse.
    let source = "---\ntitle: @daily-report\n---\n\n```\ncode\n```\n";
    session.open(&uri, source);

    let edits = session.format(&uri);

    let expected = "---\ntitle: \"@daily-report\"\n---\n\n```text\ncode\n```\n";
    assert_eq!(whole_document_edit(&edits, source), expected);
    assert_eq!(library_clean(source, root.path()), (CleanOutcome::Changed, expected.to_string()));
    assert!(session.take_messages().is_empty());
    session.shutdown();
}

#[test]
fn frontmatter_false_with_repairable_invalid_yaml_returns_no_edits_and_one_warning() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    let source = concat!(
        "---\ntitle: @daily-report\nstyle:\n  markdown:\n    frontmatter: false\n---\n",
        "\n```\ncode\n```\n"
    );
    session.open(&uri, source);

    let edits = session.format(&uri);

    assert_eq!(edits, Vec::<Value>::new());
    let warning = one_warning(&mut session);
    assert!(warning.contains("frontmatter is not valid YAML at line 2"), "{warning}");
    assert_eq!(library_clean(source, root.path()).0, CleanOutcome::Invalid);
    session.shutdown();
}

#[test]
fn workspace_rules_apply_and_the_document_overrides_them() {
    let root = tempfile::tempdir().expect("tempdir");
    let workspace_uri = file_uri(&root.path().join("workspace.md"));
    let document_uri = file_uri(&root.path().join("document.md"));
    let mut session = Session::start(root.path(), &[]);
    session.configure(json!({
        "dmls": { "formatting": { "markdown": { "list-item-padding": "none" } } }
    }));
    let loose = "- a\n\n- b\n";
    session.open(&workspace_uri, loose);
    let overridden = "---\nstyle:\n  markdown:\n    list-item-padding: one\n---\n- a\n- b\n";
    session.open(&document_uri, overridden);

    let workspace_edits = session.format(&workspace_uri);
    let document_edits = session.format(&document_uri);

    assert_eq!(whole_document_edit(&workspace_edits, loose), "- a\n- b\n");
    assert_eq!(
        whole_document_edit(&document_edits, overridden),
        "---\nstyle:\n  markdown:\n    list-item-padding: one\n---\n- a\n\n- b\n"
    );
    assert!(session.take_messages().is_empty());
    session.shutdown();
}

#[test]
fn a_legacy_setting_warns_once_per_configuration_change_not_per_request() {
    let root = tempfile::tempdir().expect("tempdir");
    let uri = file_uri(&root.path().join("doc.md"));
    let mut session = Session::start(root.path(), &[]);
    session.flush();
    assert!(session.take_messages().is_empty(), "no findings at load");

    session.configure(json!({ "dmls": { "formatting": { "cleanup": "compact" } } }));
    session.open(&uri, "- a\n\n- b\n");
    let first = session.format(&uri);
    let second = session.format(&uri);

    let messages = session.take_messages();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].0, WARNING);
    assert!(messages[0].1.contains("formatting.cleanup"), "{messages:?}");
    // The migrated value still applies: `compact` is `list-item-padding: none`.
    assert_eq!(whole_document_edit(&first, "- a\n\n- b\n"), "- a\n- b\n");
    assert_eq!(first, second);

    // The next change recomputes the findings: one warning again.
    session.configure(json!({ "dmls": { "formatting": { "cleanup": "compact" } } }));
    session.flush();
    assert_eq!(session.take_messages().len(), 1);
    session.shutdown();
}

#[test]
fn config_file_findings_are_shown_at_load_with_their_severity() {
    let root = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        root.path().join(".dmls.toml"),
        "[formatting]\ncleanup = \"loose\"\n",
    )
    .expect("write config");
    let mut session = Session::start(root.path(), &[]);
    session.flush();
    let messages = session.take_messages();
    assert_eq!(messages.len(), 1, "{messages:?}");
    assert_eq!(messages[0].0, WARNING);
    assert!(messages[0].1.contains("formatting.cleanup"), "{messages:?}");

    // Mixing the legacy and current shapes rejects the editor's settings.
    session.configure(json!({
        "dmls": { "formatting": { "cleanup": "compact", "markdown": { "unwrap": true } } }
    }));
    session.flush();
    let messages = session.take_messages();
    assert!(messages.iter().any(|(kind, _)| *kind == ERROR), "{messages:?}");
    session.shutdown();
}

#[test]
fn formatting_is_advertised_without_range_or_on_type_formatting() {
    let root = tempfile::tempdir().expect("tempdir");
    let session = Session::start(root.path(), &[]);

    assert_eq!(session.capabilities["documentFormattingProvider"], json!(true));
    assert!(session.capabilities.get("documentRangeFormattingProvider").is_none());
    assert!(session.capabilities.get("documentOnTypeFormattingProvider").is_none());
    session.shutdown();
}

#[path = "../../../lib/tests/common/clean_parity/mod.rs"]
mod clean_parity;

/// The shared parity cases through `textDocument/formatting`: the same text
/// the library transaction and `md clean` emit, no edit (with one Warning)
/// for a refusal or invalid frontmatter, and no edit or message when
/// nothing changes.
#[test]
fn every_parity_case_matches_through_formatting() {
    use clean_parity::{CASES, ParityOutcome};

    let _ = include_str!("../../../lib/tests/common/clean_parity/mod.rs");
    let root = tempfile::tempdir().expect("tempdir");
    let mut session = Session::start(root.path(), &[]);
    for case in CASES {
        let path = root.path().join(format!("{}.md", case.name));
        let uri = file_uri(&path);
        session.open(&uri, case.input);
        let edits = session.format(&uri);
        let messages = session.take_messages();
        match case.outcome {
            ParityOutcome::Changed => {
                assert_eq!(whole_document_edit(&edits, case.input), case.expected, "{}", case.name);
                assert!(messages.is_empty(), "{}: {messages:?}", case.name);
            }
            ParityOutcome::Unchanged => {
                assert!(edits.is_empty(), "{}: {edits:?}", case.name);
                assert!(messages.is_empty(), "{}: {messages:?}", case.name);
            }
            ParityOutcome::Declined | ParityOutcome::Invalid => {
                assert!(edits.is_empty(), "{}: {edits:?}", case.name);
                assert_eq!(messages.len(), 1, "{}: {messages:?}", case.name);
                assert_eq!(messages[0].0, WARNING, "{}", case.name);
            }
        }
    }
    session.shutdown();
}
