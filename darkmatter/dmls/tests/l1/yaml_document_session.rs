//! Level-1 sessions for whole YAML documents (`.yaml`, `.yml`) driven through
//! normal protocol messages: the kinds a document can select are prepared on
//! the registry worker, never the dispatch thread, and an open document is
//! refreshed when what selected or judged it changes: an unsaved edit of a
//! definition, a definition created or deleted on disk, or an edit of the
//! file its root `$schema` names. Each workspace folder's documents select
//! from that folder's own definitions. No client window is involved.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use dmls::RegistryJobKind;
use dmls::work_counters::registry_scan_threads;
use serde_json::{Value, json};

/// A user kind's definition: `owner` is a string.
const ROSTER_DEFINITION: &str = "\
kind: yaml-schema
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
    owner: string
    $constraints: { closed: true }
";

/// A valid roster.
const ROSTER: &str = "kind: roster\nroster_version: 1\nowner: ops\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`, so the
/// folder is the tree root and `schemas/` a schema root.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// A client that watches files, with `folders` as its workspace folders.
fn watched_params(folders: &[&Path]) -> Value {
    let folders: Vec<Value> = folders
        .iter()
        .map(|folder| json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "w" }))
        .collect();
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "yaml", "version": 1, "text": text } }),
    );
}

fn edit(fixture: &LspFixture<'_>, uri: &str, version: i32, text: &str) {
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": version }, "contentChanges": [{ "text": text }] }),
    );
}

/// Reports watched-file events: `(path, kind)` with 1 created, 2 changed,
/// 3 deleted.
fn changed(fixture: &LspFixture<'_>, events: &[(&Path, u8)]) {
    let changes: Vec<Value> =
        events.iter().map(|(path, kind)| json!({ "uri": uri(path), "type": kind })).collect();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
}

/// The diagnostics last published for `uri` once the server is idle; the
/// earlier publications still buffered are dropped.
fn latest_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server();
    // The buffered take returns the latest publication first.
    let latest = fixture.wait_for_diagnostics_params(uri);
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// The `dm.schema.*` codes among `diagnostics`.
fn schema_codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .filter(|code| code.starts_with("dm.schema."))
        .map(str::to_string)
        .collect()
}

fn completion_labels(fixture: &mut LspFixture<'_>, uri: &str, line: u32) -> Vec<String> {
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": 0 } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    let items = response.result.unwrap_or(Value::Null);
    let items = items.get("items").cloned().unwrap_or(items);
    items.as_array().cloned().unwrap_or_default().iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

fn hover(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> String {
    let response = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    response.result.map(|result| result["contents"]["value"].as_str().unwrap_or_default().to_string()).unwrap_or_default()
}

#[test]
fn an_unsaved_definition_edit_refreshes_an_open_document_until_the_definition_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let definition = root.join("schemas/roster.yaml");
    write(&definition, ROSTER_DEFINITION);
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());

    // The definition's buffer, edited and never saved, is what selects.
    open(&fixture, &uri(&definition), ROSTER_DEFINITION);
    edit(&fixture, &uri(&definition), 2, &ROSTER_DEFINITION.replace("owner: string", "owner: number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"]["start"], json!({ "line": 2, "character": 7 }), "{diagnostics:#?}");

    // Closing it unsaved restores the file on disk.
    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&definition) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    fixture.shutdown();
}

#[test]
fn a_definition_created_on_disk_starts_claiming_and_deleting_it_clears_the_results() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/notes.yaml"), "just: data\n");
    let document = root.join("team.yaml");
    let text = "kind: roster\nroster_version: 1\nowner: [a]\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    // No definition claims `roster` yet: the document is not validated.
    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(!completion_labels(&mut fixture, &uri(&document), 3).contains(&"owner".to_string()));

    let definition = root.join("schemas/roster.yaml");
    write(&definition, ROSTER_DEFINITION);
    changed(&fixture, &[(&definition, 1)]);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    let body = hover(&mut fixture, &uri(&document), 1, 2);
    assert!(body.contains("roster_version"), "{body}");

    std::fs::remove_file(&definition).unwrap();
    changed(&fixture, &[(&definition, 3)]);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "no stale hover");
    fixture.shutdown();
}

#[test]
fn an_edited_root_schema_file_refreshes_the_document_that_names_it() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let contract = root.join("contract.yaml");
    write(&contract, "$schema:\n    port: number\n");
    let document = root.join("service.yaml");
    let text = "$schema: ./contract.yaml\nport: http\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), vec!["dm.schema.type_mismatch".to_string()]);

    write(&contract, "$schema:\n    port: string\n");
    changed(&fixture, &[(&contract, 2)]);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());

    std::fs::remove_file(&contract).unwrap();
    changed(&fixture, &[(&contract, 3)]);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics).len(), 1, "a deleted contract is a prepare error: {diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn each_workspace_folder_selects_from_its_own_definitions() {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    for (folder, owner) in [(&first, "string"), (&second, "number")] {
        init_repository(folder);
        write(&folder.join("schemas/roster.yaml"), &ROSTER_DEFINITION.replace("owner: string", &format!("owner: {owner}")));
        write(&folder.join("team.yaml"), ROSTER);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&first, &second]));

    open(&fixture, &uri(&first.join("team.yaml")), ROSTER);
    open(&fixture, &uri(&second.join("team.yaml")), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&first.join("team.yaml")))), Vec::<String>::new());
    assert_eq!(
        schema_codes(&latest_diagnostics(&mut fixture, &uri(&second.join("team.yaml")))),
        vec!["dm.schema.type_mismatch".to_string()]
    );
    fixture.shutdown();
}

#[test]
fn a_yaml_documents_kinds_are_prepared_off_the_dispatch_thread() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/roster.yaml"), ROSTER_DEFINITION);
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    let dispatch = fixture.server_thread_id();
    assert_ne!(fixture.wait_for_registry_prepared(RegistryJobKind::Document), dispatch);
    let threads = registry_scan_threads();
    assert!(!threads.contains(&dispatch), "a registry scan ran on the dispatch thread: {threads:?}");
    // The prepared kinds judge the document: an error proves the
    // definition was found by that preparation.
    edit(&fixture, &uri(&document), 2, &ROSTER.replace("roster_version: 1", "roster_version: 2"));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), vec!["dm.schema.constraint".to_string()]);
    fixture.shutdown();
}
