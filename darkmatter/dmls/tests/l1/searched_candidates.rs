//! Level-1 sessions for searched payload candidates: a trigger payload's
//! import `Fields@shared/types.yaml` resolves in the authoring folder first
//! (`schemas/shared/types.yaml`, absent with its folder) and then at the
//! repository root. A ready registry must notice the nearer file through
//! the client's event for it (a watched event or a save) or a fallback
//! rescan, without a test-only invalidation, and the absent folder must be
//! watched from its nearest existing ancestor until it exists.
//!
//! The work counters are process-wide; nextest runs each test in its own
//! process.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use dmls::work_counters::work_counts;
use dmls::{StartupEvent, StartupPhase};
use serde_json::{Value, json};

const ENVELOPE: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema: prompt.yaml\n";
const PAYLOAD: &str = "$schema:\n  model: Fields@shared/types.yaml\n";
const SHARED_FIELDS: &str = "$schema:\n  Fields: string\n";
const NEARER_FIELDS: &str = "$schema:\n  Fields: number\n";
const MALFORMED_FIELDS: &str = "$schema:\n  Fields: [unclosed\n";
const DOC: &str = "---\nprompt: hello\nmodel: text\n---\n\nBody.\n";

const CREATED: u32 = 1;
const DELETED: u32 = 3;

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`, whose
/// trigger matches `doc.md` and whose payload imports the shared `Fields`.
/// Returns the canonical repository root, the spelling DMLS records.
fn candidate_workspace() -> (LspWorkspace, PathBuf) {
    let workspace = LspWorkspace::new();
    let root = biscuit_file::canonicalize_simplified(workspace.path()).unwrap();
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
    write(&root.join("schemas/prompt.trigger.yaml"), ENVELOPE);
    write(&root.join("schemas/prompt.yaml"), PAYLOAD);
    write(&root.join("shared/types.yaml"), SHARED_FIELDS);
    write(&root.join("doc.md"), DOC);
    (workspace, root)
}

fn nearer(root: &Path) -> PathBuf {
    root.join("schemas/shared/types.yaml")
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// A client that watches files and roots watchers at folders.
fn folder_watch_params(root: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Candidate Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true } }
        },
        "workspaceFolders": [ { "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "r" } ]
    })
}

/// A client with no file watcher, so DMLS rescans on save.
fn rescan_params(root: &Path) -> Value {
    let mut params = folder_watch_params(root);
    params["clientInfo"] = json!({ "name": "Neovim", "version": "0.11.0" });
    params["capabilities"]["workspace"].as_object_mut().unwrap().remove("didChangeWatchedFiles");
    params
}

fn open(fixture: &LspFixture<'_>, root: &Path) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&root.join("doc.md")), "languageId": "markdown", "version": 1, "text": DOC } }),
    );
}

fn watched(fixture: &LspFixture<'_>, path: &Path, kind: u32) {
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(path), "type": kind } ] }));
}

fn saved(fixture: &LspFixture<'_>, path: &Path) {
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(path) } }));
}

/// The diagnostics last published for `uri` once the server has handled
/// every earlier message and finished the registry jobs they queued.
fn latest(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": uri } }));
    let latest = fixture.take_buffered_diagnostics(uri).expect("diagnostics were published");
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// Whether the document's `model: text` is reported, which only the nearer
/// file's `Fields: number` does.
fn rejects_model(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|diagnostic| {
        diagnostic["code"] == json!("dm.schema.type_mismatch") && diagnostic["data"]["instance_path"] == json!("/model")
    })
}

fn has_prepare_error(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare"))
}

/// The `(base URI, pattern)` pairs of every schema-location registration
/// the server has sent, oldest first. Read after [`latest`], whose flush
/// buffers them.
fn schema_registrations(fixture: &LspFixture<'_>) -> Vec<Vec<(String, String)>> {
    fixture
        .buffered_server_requests()
        .iter()
        .filter(|request| request.method == "client/registerCapability")
        .flat_map(|request| request.params["registrations"].as_array().cloned().unwrap_or_default())
        .filter(|registration| registration["id"] == json!("dmls-watch-schema-inputs"))
        .map(|registration| {
            let watchers = registration["registerOptions"]["watchers"].as_array().cloned().unwrap_or_default();
            watchers
                .iter()
                .map(|watcher| {
                    let pattern = &watcher["globPattern"];
                    (
                        pattern["baseUri"].as_str().unwrap_or_default().trim_end_matches('/').to_string(),
                        pattern["pattern"].as_str().unwrap_or_default().to_string(),
                    )
                })
                .collect()
        })
        .collect()
}

fn watch(base: &Path, pattern: &str) -> (String, String) {
    (url::Url::from_file_path(base).unwrap().as_str().trim_end_matches('/').to_string(), pattern.to_string())
}

#[test]
fn a_watched_nearer_candidate_reselects_the_import_and_its_removal_falls_back() {
    let (workspace, root) = candidate_workspace();
    let doc = uri(&root.join("doc.md"));
    let schemas = root.join("schemas");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(folder_watch_params(&root));
    open(&fixture, &root);
    let initial = latest(&mut fixture, &doc);
    assert!(!rejects_model(&initial), "the root `Fields: string` accepts text: {initial:#?}");
    let registrations = schema_registrations(&fixture);
    let watches = registrations.last().expect("schema locations are watched");
    for expected in [watch(&schemas, "shared"), watch(&schemas, "shared/*")] {
        assert!(watches.contains(&expected), "the absent folder is watched from `schemas/`: {watches:?}");
    }

    write(&nearer(&root), NEARER_FIELDS);
    watched(&fixture, &nearer(&root), CREATED);
    let shadowed = latest(&mut fixture, &doc);
    assert!(rejects_model(&shadowed), "the nearer `Fields: number` is selected: {shadowed:#?}");
    let advanced = schema_registrations(&fixture);
    assert_eq!(advanced.len(), registrations.len() + 1, "the watch set changed, so it is registered again");
    let advanced = advanced.last().unwrap();
    assert!(advanced.contains(&watch(&schemas, "shared/*")), "{advanced:?}");
    assert!(!advanced.contains(&watch(&schemas, "shared")), "the folder exists now: {advanced:?}");

    std::fs::remove_file(nearer(&root)).unwrap();
    watched(&fixture, &nearer(&root), DELETED);
    let fallback = latest(&mut fixture, &doc);
    assert!(!rejects_model(&fallback), "the root file is selected again: {fallback:#?}");
    fixture.shutdown();
}

#[test]
fn a_saved_nearer_candidate_reselects_the_import() {
    let (workspace, root) = candidate_workspace();
    let doc = uri(&root.join("doc.md"));
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(folder_watch_params(&root));
    open(&fixture, &root);
    assert!(!rejects_model(&latest(&mut fixture, &doc)));

    write(&nearer(&root), NEARER_FIELDS);
    saved(&fixture, &nearer(&root));
    let shadowed = latest(&mut fixture, &doc);
    assert!(rejects_model(&shadowed), "{shadowed:#?}");
    fixture.shutdown();
}

#[test]
fn a_fallback_rescan_rebuilds_from_the_new_candidate() {
    let (workspace, root) = candidate_workspace();
    let doc = uri(&root.join("doc.md"));
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(rescan_params(&root));
    open(&fixture, &root);
    assert!(!rejects_model(&latest(&mut fixture, &doc)));

    // No client event reports the new file; saving any document rescans.
    write(&nearer(&root), NEARER_FIELDS);
    saved(&fixture, &root.join("doc.md"));
    let shadowed = latest(&mut fixture, &doc);
    assert!(rejects_model(&shadowed), "{shadowed:#?}");
    fixture.shutdown();
}

/// Startup's preparation has searched for `schemas/shared/types.yaml` (and
/// not found it) when a malformed file appears there and its event arrives.
/// The replayed change leaves startup's scope stale, so the refresh the
/// event scheduled prepares it again and reports the malformed import on
/// the trigger; were the outdated reading installed as fresh, nothing would
/// be published. Repairing the file clears the report.
#[test]
fn a_candidate_created_during_an_in_flight_preparation_is_prepared_again_and_recovers() {
    let (workspace, root) = candidate_workspace();
    let trigger = uri(&root.join("schemas/prompt.trigger.yaml"));
    let mut fixture = LspFixture::start_with_startup_probe_gate(&workspace);
    fixture.initialize_unlisted(folder_watch_params(&root));
    fixture.startup_events_until(|event| *event == StartupEvent::StartupProbePreparing(StartupPhase::Initial));

    write(&nearer(&root), MALFORMED_FIELDS);
    watched(&fixture, &nearer(&root), CREATED);
    fixture.round_trip();
    assert!(fixture.take_buffered_diagnostics(&trigger).is_none(), "nothing prepared the change yet");

    fixture.open_startup_probe_gate();
    let failed = latest(&mut fixture, &trigger);
    assert!(has_prepare_error(&failed), "the malformed nearer file is reported: {failed:#?}");

    write(&nearer(&root), NEARER_FIELDS);
    watched(&fixture, &nearer(&root), 2);
    let repaired = latest(&mut fixture, &trigger);
    assert!(!has_prepare_error(&repaired), "the repair recovers: {repaired:#?}");
    fixture.shutdown();
}

#[test]
fn unrelated_events_and_ready_requests_prepare_nothing() {
    let (workspace, root) = candidate_workspace();
    let doc = uri(&root.join("doc.md"));
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(folder_watch_params(&root));
    open(&fixture, &root);
    latest(&mut fixture, &doc);

    // Same file name as the candidate, in folders nothing searched.
    let unrelated = [root.join("elsewhere/shared/types.yaml"), root.join("schemas-old/shared/types.yaml")];
    for path in &unrelated {
        write(path, NEARER_FIELDS);
    }
    let before = work_counts();
    for path in &unrelated {
        watched(&fixture, path, CREATED);
    }
    for _ in 0..3 {
        let hover = fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": doc }, "position": { "line": 1, "character": 1 } }),
        );
        assert!(hover.error.is_none(), "{hover:?}");
    }
    fixture.flush_server();
    let work = work_counts().since(before);
    assert_eq!((work.registry_scans, work.enumerations), (0, 0), "{work:?}");
    fixture.shutdown();
}
