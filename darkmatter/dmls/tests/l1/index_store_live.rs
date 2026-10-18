//! Level-1 sessions for the index store behind the server: the first
//! references request opens and fills the store, rows follow disk changes
//! per watch mode, and a rename re-reads what it edits. Rows are read back
//! with a second connection to the store the session holds open.
//!
//! Every session sets `DMLS_CACHE_DIR` to a directory inside its workspace,
//! so no test writes to the developer's cache.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::common;

use common::{LspFixture, LspWorkspace};
use dmls::StartupEvent;
use dmls::clock::SystemClock;
use dmls::index_store::{IndexStore, store_path};
use serde_json::{Value, json};

const TARGET: &str = "# Target\n\nBody.\n";
const REFERRER: &str = "# Referrer\n\nSee [the target](target.md#target).\n";
const UNRELATED: &str = "# Other\n\nNothing here.\n";

fn initialize_params(root: &Path, watched: bool) -> Value {
    let root_uri = url::Url::from_directory_path(root).unwrap();
    let workspace = if watched {
        json!({ "didChangeWatchedFiles": { "dynamicRegistration": true } })
    } else {
        json!({})
    };
    json!({
        "processId": null,
        "clientInfo": { "name": "Store Client", "version": "1.0" },
        "capabilities": { "general": { "positionEncodings": ["utf-16"] }, "workspace": workspace },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "store" } ]
    })
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn write(workspace: &LspWorkspace, name: &str, text: &str) -> PathBuf {
    let path = workspace.path().join(name);
    std::fs::write(&path, text).unwrap();
    path
}

fn cache_dir(workspace: &LspWorkspace) -> PathBuf {
    workspace.path().join("cache")
}

/// A session whose stores live in [`cache_dir`] (or are `off`).
fn start<'w>(workspace: &'w LspWorkspace, cache: &str, watched: bool) -> LspFixture<'w> {
    let env = HashMap::from([("DMLS_CACHE_DIR".to_string(), cache.to_string())]);
    let snapshot = LspFixture::fixture_snapshot(workspace).with_env(env);
    let mut fixture = LspFixture::start_with_snapshot(workspace, snapshot);
    fixture.initialize(initialize_params(workspace.path(), watched));
    fixture
}

fn start_cached<'w>(workspace: &'w LspWorkspace, watched: bool) -> LspFixture<'w> {
    start(workspace, cache_dir(workspace).to_str().unwrap(), watched)
}

/// The workspace with `target.md`, a never-opened `referrer.md` linking its
/// heading, and an unrelated `other.md`.
fn workspace() -> LspWorkspace {
    let workspace = LspWorkspace::new();
    write(&workspace, "target.md", TARGET);
    write(&workspace, "referrer.md", REFERRER);
    write(&workspace, "other.md", UNRELATED);
    workspace
}

/// Waits until the server has handled every message sent so far, with a
/// request that does not need the store (the fixture's own round trip is
/// `workspace/symbol`, which does). Every session
/// keeps `target.md` open.
fn settle(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace) {
    let target = uri(&workspace.path().join("target.md"));
    let response = fixture.request("textDocument/documentSymbol", json!({ "textDocument": { "uri": target } }));
    assert!(response.error.is_none(), "{:?}", response.error);
}

fn open(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, path: &Path, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(path), "languageId": "markdown", "version": 1, "text": text } }),
    );
    settle(fixture, workspace);
}

fn watched(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, path: &Path, kind: u32) {
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(path), "type": kind } ] }));
    settle(fixture, workspace);
}

fn save(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, path: &Path) {
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(path) } }));
    settle(fixture, workspace);
}

/// The file names a references request on `target.md`'s heading answers.
fn references(fixture: &mut LspFixture<'_>, target: &Path) -> Vec<String> {
    let response = fixture.request(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri(target) },
            "position": { "line": 0, "character": 3 },
            "context": { "includeDeclaration": false }
        }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    let mut files: Vec<String> = response.result.unwrap().as_array().unwrap()
        .iter()
        .map(|location| location["uri"].as_str().unwrap().rsplit('/').next().unwrap().to_string())
        .collect();
    files.sort();
    files
}

/// `file name -> [(start line, new text)]` of a workspace edit.
fn edits(result: &Value) -> BTreeMap<String, Vec<(u64, String)>> {
    let mut out = BTreeMap::new();
    for (uri, edits) in result["changes"].as_object().expect("a `changes` edit") {
        let name = uri.rsplit('/').next().unwrap().to_string();
        let mut list: Vec<(u64, String)> = edits
            .as_array()
            .unwrap()
            .iter()
            .map(|edit| (edit["range"]["start"]["line"].as_u64().unwrap(), edit["newText"].as_str().unwrap().to_string()))
            .collect();
        list.sort();
        out.insert(name, list);
    }
    out
}

fn rename_heading(fixture: &mut LspFixture<'_>, target: &Path, new_name: &str) -> BTreeMap<String, Vec<(u64, String)>> {
    let response = fixture.request(
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri(target) }, "position": { "line": 0, "character": 3 }, "newName": new_name }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    edits(&response.result.unwrap())
}

fn rename_file(fixture: &mut LspFixture<'_>, from: &Path, to: &Path) -> BTreeMap<String, Vec<(u64, String)>> {
    let response = fixture.request(
        "workspace/willRenameFiles",
        json!({ "files": [ { "oldUri": uri(from), "newUri": uri(to) } ] }),
    );
    assert!(response.error.is_none(), "{:?}", response.error);
    edits(&response.result.unwrap())
}

/// Every row of the workspace root's store and its hash, read through a
/// second connection.
fn rows(workspace: &LspWorkspace) -> BTreeMap<String, u64> {
    let root = workspace.path();
    let path = store_path(&cache_dir(workspace), root);
    let (store, _) = IndexStore::open(&path, root, Arc::new(SystemClock)).expect("the session's store opens");
    store
        .hashes()
        .unwrap()
        .into_iter()
        .map(|(path, hash)| (path.file_name().unwrap().to_string_lossy().into_owned(), hash))
        .collect()
}

fn hash_of(text: &str) -> u64 {
    biscuit_hash::xx_hash_bytes(text.as_bytes())
}

fn store_files(workspace: &LspWorkspace) -> Vec<String> {
    match std::fs::read_dir(cache_dir(workspace)) {
        Ok(entries) => entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => Vec::new(),
    }
}

/// Nothing opens a store until a request needs one; the first references
/// request fills it with a row per listed file whose hash is its bytes.
#[test]
fn the_first_references_request_fills_the_store_from_disk() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    assert!(store_files(&workspace).iter().all(|name| !name.ends_with(".sqlite")), "{:?}", store_files(&workspace));

    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    assert_eq!(
        rows(&workspace),
        BTreeMap::from([
            ("other.md".to_string(), hash_of(UNRELATED)),
            ("referrer.md".to_string(), hash_of(REFERRER)),
            ("target.md".to_string(), hash_of(TARGET)),
        ])
    );
    fixture.shutdown();
}

/// `DMLS_CACHE_DIR=off`: the requests answer from memory and no store file
/// is created.
#[test]
fn the_kill_switch_answers_from_memory_and_writes_no_store() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let mut fixture = start(&workspace, "off", true);
    open(&mut fixture, &workspace, &target, TARGET);
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    fixture.shutdown();
    assert!(store_files(&workspace).is_empty(), "{:?}", store_files(&workspace));
}

/// A store file that is not a database is recreated, and the request still
/// answers.
#[test]
fn a_corrupt_store_is_recreated_and_the_request_answers() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    std::fs::create_dir(cache_dir(&workspace)).unwrap();
    std::fs::write(store_path(&cache_dir(&workspace), workspace.path()), b"this is not a database").unwrap();
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    assert_eq!(rows(&workspace).len(), 3);
    fixture.shutdown();
}

/// R6 before ready: a watched change reaches only the file list; the first
/// need then reads the new bytes.
#[test]
fn before_ready_a_watched_change_touches_only_the_file_list() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let other = workspace.path().join("other.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    let changed = "# Other\n\n[t](target.md#target)\n";
    std::fs::write(&other, changed).unwrap();
    watched(&mut fixture, &workspace, &other, 2);
    assert!(store_files(&workspace).iter().all(|name| !name.ends_with(".sqlite")));

    assert_eq!(references(&mut fixture, &target), vec!["other.md".to_string(), "referrer.md".to_string()]);
    assert_eq!(rows(&workspace)["other.md"], hash_of(changed));
    fixture.shutdown();
}

/// R6 after ready, client watching: an event re-reads and upserts exactly
/// its file with one parse (the closed file's stale index parse is not
/// re-read, only marked), and a deletion removes the row.
#[test]
fn after_ready_a_watched_event_updates_one_row() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let other = workspace.path().join("other.md");
    let referrer = workspace.path().join("referrer.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    references(&mut fixture, &target);
    let before = rows(&workspace);

    let changed = "# Other, edited\n";
    std::fs::write(&other, changed).unwrap();
    let work = dmls::work_counters::work_counts();
    watched(&mut fixture, &workspace, &other, 2);
    let work = dmls::work_counters::work_counts().since(work);
    assert_eq!(work.parses, 1, "only the store parses the new bytes: {work:?}");
    let after = rows(&workspace);
    assert_eq!(after["other.md"], hash_of(changed));
    assert_eq!((after["referrer.md"], after["target.md"]), (before["referrer.md"], before["target.md"]));

    std::fs::remove_file(&referrer).unwrap();
    watched(&mut fixture, &workspace, &referrer, 3);
    assert!(!rows(&workspace).contains_key("referrer.md"));
    fixture.shutdown();
}

/// A change that arrives while a build round reads is re-read by the next
/// round, so the ready store never holds the bytes the round saw.
#[test]
fn a_change_during_a_build_round_is_reread_before_ready() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let other = workspace.path().join("other.md");
    let env = HashMap::from([("DMLS_CACHE_DIR".to_string(), cache_dir(&workspace).to_string_lossy().into_owned())]);
    let mut fixture = LspFixture::start_with_index_round_gate_and_snapshot(
        &workspace,
        LspFixture::fixture_snapshot(&workspace).with_env(env),
    );
    fixture.initialize(initialize_params(workspace.path(), true));
    open(&mut fixture, &workspace, &target, TARGET);

    let id = lsp_server::RequestId::from("held".to_string());
    fixture.send_request_with_id(
        id.clone(),
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri(&target) },
            "position": { "line": 0, "character": 3 },
            "context": { "includeDeclaration": false }
        }),
    );
    fixture.wait_for_event(StartupEvent::IndexRoundRead);
    let changed = "# Other\n\n[t](target.md#target)\n";
    std::fs::write(&other, changed).unwrap();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(&other), "type": 2 } ] }));
    fixture.open_index_round_gate();
    fixture.release_index_round();
    let response = fixture.expect_response(id);
    assert!(response.error.is_none(), "{:?}", response.error);
    assert_eq!(rows(&workspace)["other.md"], hash_of(changed));
    fixture.shutdown();
}

/// R6 fallback watch mode, after ready: a save re-reads the saved file and
/// nothing else, so a file changed outside the editor keeps its old row
/// until something re-reads it.
#[test]
fn a_fallback_save_rereads_only_the_saved_file() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let other = workspace.path().join("other.md");
    let mut fixture = start_cached(&workspace, false);
    open(&mut fixture, &workspace, &target, TARGET);
    references(&mut fixture, &target);
    let before = rows(&workspace);

    let saved = "# Target\n\nSaved.\n";
    std::fs::write(&target, saved).unwrap();
    std::fs::write(&other, "# Other, changed outside the editor\n").unwrap();
    save(&mut fixture, &workspace, &target);
    let after = rows(&workspace);
    assert_eq!(after["target.md"], hash_of(saved));
    assert_eq!(after["other.md"], before["other.md"], "no event named it");
    fixture.shutdown();
}

/// Acceptance 7: in fallback watch mode a heading rename after an unwatched
/// on-disk change that added a referrer edits that referrer, and so does a
/// file rename.
#[test]
fn a_fallback_rename_edits_a_referrer_added_on_disk_without_an_event() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let other = workspace.path().join("other.md");
    let mut fixture = start_cached(&workspace, false);
    open(&mut fixture, &workspace, &target, TARGET);
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);

    std::fs::write(&other, "# Other\n\nNow [links](target.md#target).\n").unwrap();
    let renamed = rename_heading(&mut fixture, &target, "Goal");
    assert!(renamed.contains_key("other.md"), "{renamed:?}");
    assert!(renamed.contains_key("referrer.md"), "{renamed:?}");
    assert_eq!(rows(&workspace)["other.md"], hash_of("# Other\n\nNow [links](target.md#target).\n"));

    // A path link (file rename edits path links; one with a heading
    // fragment is not among them, on disk or here).
    std::fs::write(&other, "# Other\n\nNow [a path link](target.md).\n").unwrap();
    let moved = rename_file(&mut fixture, &target, &workspace.path().join("goal.md"));
    assert_eq!(moved.get("other.md"), Some(&vec![(2, "goal.md".to_string())]), "{moved:?}");
    fixture.shutdown();
}

/// Every rename re-reads the referrers it edits: a referrer whose lines
/// moved on disk after ready, with no event, is edited at its new lines.
#[test]
fn a_rename_edits_the_new_bytes_of_a_referrer_changed_after_ready() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let referrer = workspace.path().join("referrer.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    references(&mut fixture, &target);

    // The link moves from line 2 to line 4.
    std::fs::write(&referrer, "# Referrer\n\nAdded.\n\nSee [the target](target.md#target).\n").unwrap();
    let renamed = rename_heading(&mut fixture, &target, "Goal");
    assert_eq!(renamed["referrer.md"].iter().map(|(line, _)| *line).collect::<Vec<_>>(), vec![4], "{renamed:?}");
    fixture.shutdown();
}

/// Acceptance 1 through the server: a restart over an unchanged workspace
/// reads and hashes every listed file and parses none of them before the
/// first references answer, which still includes the never-opened referrer.
#[test]
fn a_warm_restart_parses_nothing_before_the_first_references_answer() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    fixture.shutdown();

    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    let before = dmls::work_counters::work_counts();
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    let work = dmls::work_counters::work_counts().since(before);
    assert_eq!(work.parses, 0, "{work:?}");
    assert_eq!(work.reads, 3, "every listed file is read and hashed once: {work:?}");
    fixture.shutdown();
}

/// Acceptance 2 through the server: one file changed between restarts is
/// the only parse, and its new link is answered.
#[test]
fn one_file_changed_between_restarts_is_the_only_parse() {
    let workspace = workspace();
    let target = workspace.path().join("target.md");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    assert_eq!(references(&mut fixture, &target), vec!["referrer.md".to_string()]);
    fixture.shutdown();

    write(&workspace, "other.md", "# Other\n\nNow [a link](target.md#target).\n");
    let mut fixture = start_cached(&workspace, true);
    open(&mut fixture, &workspace, &target, TARGET);
    let before = dmls::work_counters::work_counts();
    assert_eq!(references(&mut fixture, &target), vec!["other.md".to_string(), "referrer.md".to_string()]);
    let work = dmls::work_counters::work_counts().since(before);
    assert_eq!(work.parses, 1, "{work:?}");
    fixture.shutdown();
}
