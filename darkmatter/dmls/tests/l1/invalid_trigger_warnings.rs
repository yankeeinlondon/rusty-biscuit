//! Level-1 sessions for schema triggers discovery skips as invalid: the
//! registry installs the current valid triggers, so a trigger saved invalid
//! stops contributing to validation, completion, and hover while a valid
//! neighbor's changes apply; each skipped trigger file gets one warning
//! diagnostic, which an unchanged refresh never republishes and a repair or
//! deletion clears; and an open trigger keeps its strict authoring error.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

/// The incident shape: unsupported envelope keys and a null `match`.
const INCIDENT_TRIGGER: &str =
    "kind: schema-trigger\nname: memory\ndescription: Memory notes\nmatch:\n$schema: alpha.yaml\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`, so the
/// workspace is the tree root and `schemas/` a schema root.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// A client that watches files.
fn watched_params(root: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": [{ "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "w" }]
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, language: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": language, "version": 1, "text": text } }),
    );
}

/// Reports watched-file events: `(path, kind)` with 1 created, 2 changed,
/// 3 deleted.
fn changed(fixture: &LspFixture<'_>, events: &[(&Path, u8)]) {
    let changes: Vec<Value> =
        events.iter().map(|(path, kind)| json!({ "uri": uri(path), "type": kind })).collect();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
}

/// The diagnostics last published for `uri` once the server is idle.
fn latest_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server();
    let mut latest = fixture.wait_for_diagnostics_params(uri);
    while let Some(params) = fixture.take_buffered_diagnostics(uri) {
        latest = params;
    }
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn invalid_trigger_warnings(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic["code"] == json!("dm.schema.invalid_trigger")
                && diagnostic["severity"] == json!(2)
                && diagnostic["source"] == json!("darkmatter.schema")
                && diagnostic["message"]
                    .as_str()
                    .is_some_and(|message| message.starts_with("ignoring invalid schema trigger "))
        })
        .collect()
}

fn requires(diagnostics: &[Value], property: &str) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == json!("dm.schema.missing_required")
            && diagnostic["message"].as_str().is_some_and(|message| message.contains(property))
    })
}

fn completion_labels(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<String> {
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 2, "character": 0 } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    let items = response.result.unwrap_or(Value::Null).as_array().cloned().unwrap_or_default();
    items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

fn hover(fixture: &mut LspFixture<'_>, uri: &str) -> String {
    let response = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": 2, "character": 2 } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    response.result.unwrap_or(Value::Null).to_string()
}

/// Two triggers matching `prompt`: A contributes `alpha` (a date), B `beta`.
/// Strict mode, so DMLS reports the triggered schema's required properties.
fn two_trigger_fixture(root: &Path) {
    init_repository(root);
    write(&root.join(".dmls.toml"), "[schema]\nstrict = true\n");
    write(
        &root.join("schemas/a.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: alpha.yaml\n",
    );
    write(&root.join("schemas/alpha.yaml"), "$schema:\n  alpha: date(required)\n");
    write(
        &root.join("schemas/b.trigger.yaml"),
        "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: beta.yaml\n",
    );
    write(&root.join("schemas/beta.yaml"), "$schema:\n  beta: string(required)\n");
}

#[test]
fn a_trigger_saved_invalid_loses_its_properties_while_a_valid_neighbor_changes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    two_trigger_fixture(root);
    let document_path = root.join("prompt.md");
    let document = "---\nprompt: hello\nalpha: 2024-01-01\n---\n\nBody\n";
    write(&document_path, document);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(root));
    let document_uri = uri(&document_path);
    open(&fixture, &document_uri, "markdown", document);

    let initial = latest_diagnostics(&mut fixture, &document_uri);
    assert!(requires(&initial, "beta"), "{initial:#?}");
    let labels = completion_labels(&mut fixture, &document_uri);
    assert!(labels.iter().any(|label| label == "beta"), "{labels:?}");
    let declared = hover(&mut fixture, &document_uri);
    assert!(declared.contains("date"), "hover shows A's declared type: {declared}");

    // A is saved in the incident shape while B's payload changes `beta` to
    // `gamma`.
    let envelope_a = root.join("schemas/a.trigger.yaml");
    let payload_b = root.join("schemas/beta.yaml");
    write(&envelope_a, INCIDENT_TRIGGER);
    write(&payload_b, "$schema:\n  gamma: string(required)\n");
    changed(&fixture, &[(&envelope_a, 2), (&payload_b, 2)]);

    let warned = fixture.wait_for_diagnostics(&uri(&envelope_a));
    let warnings = invalid_trigger_warnings(&warned);
    assert_eq!(warnings.len(), 1, "{warned:#?}");
    assert!(warnings[0]["message"].as_str().unwrap().contains("a.trigger.yaml"), "{warned:#?}");
    let current = latest_diagnostics(&mut fixture, &document_uri);
    assert!(requires(&current, "gamma"), "the neighbor's change applies: {current:#?}");
    assert!(!requires(&current, "beta"), "{current:#?}");
    assert!(
        current.iter().all(|diagnostic| diagnostic["code"] != json!("dm.schema.prepare")),
        "a skipped trigger never fails the document: {current:#?}"
    );
    let labels = completion_labels(&mut fixture, &document_uri);
    assert!(labels.iter().any(|label| label == "gamma"), "{labels:?}");
    assert!(!labels.iter().any(|label| label == "beta"), "{labels:?}");
    let undeclared = hover(&mut fixture, &document_uri);
    assert!(!undeclared.contains("date"), "A's declared type is gone from hover: {undeclared}");

    // Deleting the skipped trigger clears its warning.
    std::fs::remove_file(&envelope_a).unwrap();
    changed(&fixture, &[(&envelope_a, 3)]);
    let cleared = latest_diagnostics(&mut fixture, &uri(&envelope_a));
    assert!(cleared.is_empty(), "{cleared:#?}");
    fixture.shutdown();
}

#[test]
fn a_first_load_with_only_invalid_triggers_warns_on_each_and_an_unchanged_refresh_publishes_nothing() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let incident = root.join("schemas/memory.yaml");
    let unparseable = root.join("schemas/notes.trigger.yaml");
    let unparseable_payload = root.join("schemas/notes.yaml");
    write(&incident, INCIDENT_TRIGGER);
    write(&unparseable, "kind: schema-trigger\nmatch:\n  notes: string(required)\n$schema: notes.yaml\n");
    write(&unparseable_payload, "$schema:\n  notes: [unclosed\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(root));

    // Published at startup, with no document open.
    let incident_diagnostics = fixture.wait_for_diagnostics(&uri(&incident));
    assert_eq!(invalid_trigger_warnings(&incident_diagnostics).len(), 1, "{incident_diagnostics:#?}");
    let payload_diagnostics = fixture.wait_for_diagnostics(&uri(&unparseable));
    let payload_warnings = invalid_trigger_warnings(&payload_diagnostics);
    assert_eq!(payload_warnings.len(), 1, "{payload_diagnostics:#?}");
    assert_eq!(
        payload_warnings[0]["relatedInformation"][0]["location"]["uri"],
        json!(uri(&unparseable_payload)),
        "the payload the cause points into is related information: {payload_diagnostics:#?}"
    );

    // A document in that scope composes against the baseline alone.
    let document_path = root.join("doc.md");
    let document = "---\nnotes: hello\n---\n\nBody\n";
    write(&document_path, document);
    open(&fixture, &uri(&document_path), "markdown", document);
    let document_diagnostics = latest_diagnostics(&mut fixture, &uri(&document_path));
    assert!(
        document_diagnostics.iter().all(|diagnostic| {
            diagnostic["code"] != json!("dm.schema.prepare") && diagnostic["code"] != json!("dm.schema.invalid_trigger")
        }),
        "the warning stays on the trigger file: {document_diagnostics:#?}"
    );

    // An event for an unchanged file republishes nothing.
    changed(&fixture, &[(&incident, 2), (&unparseable, 2)]);
    fixture.flush_server();
    assert!(fixture.take_buffered_diagnostics(&uri(&incident)).is_none());
    assert!(fixture.take_buffered_diagnostics(&uri(&unparseable)).is_none());

    // Repairing the payload clears only its trigger's warning.
    write(&unparseable_payload, "$schema:\n  title: string\n");
    changed(&fixture, &[(&unparseable_payload, 2)]);
    let repaired = latest_diagnostics(&mut fixture, &uri(&unparseable));
    assert!(repaired.is_empty(), "{repaired:#?}");
    assert!(fixture.take_buffered_diagnostics(&uri(&incident)).is_none(), "the other warning stays published");
    fixture.shutdown();
}

#[test]
fn an_open_trigger_stays_strict_while_edited_and_keeps_its_warning_once_closed() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let envelope = root.join("schemas/late.trigger.yaml");
    let text = "kind: schema-trigger\nmatch:\n  late: string(required)\n$schema: missing.yaml\n";
    write(&envelope, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(root));
    let envelope_uri = uri(&envelope);
    let _ = fixture.wait_for_diagnostics(&envelope_uri);

    // Open with an envelope that parses: the saved trigger's warning.
    open(&fixture, &envelope_uri, "yaml", text);
    let opened = latest_diagnostics(&mut fixture, &envelope_uri);
    assert_eq!(invalid_trigger_warnings(&opened).len(), 1, "{opened:#?}");

    // A malformed unsaved edit gets the strict authoring error.
    fixture.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": envelope_uri, "version": 2 },
            "contentChanges": [{ "text": "kind: schema-trigger\nmatch: [unclosed\n" }]
        }),
    );
    let edited = latest_diagnostics(&mut fixture, &envelope_uri);
    assert!(
        edited.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare") && diagnostic["severity"] == json!(1)),
        "{edited:#?}"
    );

    // Closing it keeps the saved trigger's warning on the file.
    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": envelope_uri } }));
    let closed = latest_diagnostics(&mut fixture, &envelope_uri);
    assert_eq!(invalid_trigger_warnings(&closed).len(), 1, "{closed:#?}");
    fixture.shutdown();
}
