//! Level-1 sessions for the requests the index store answers, across a
//! restart and across workspace roots.
//!
//! Each test runs a first session that fills the stores and shuts down,
//! then a second session over the same `DMLS_CACHE_DIR` that opens only
//! the document a request starts from. The referrers the second session
//! edits or lists were never open in it and are not referenced by an open
//! document, so only the stored facts name them.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The minimal `.git` a file-resolution context recognizes as a repository.
fn init_repository(root: &Path) {
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
}

/// A session over the workspace `folders`, with its stores in the
/// workspace's `cache` directory.
fn start<'w>(workspace: &'w LspWorkspace, folders: &[&Path]) -> LspFixture<'w> {
    let cache = workspace.path().join("cache");
    let env = HashMap::from([("DMLS_CACHE_DIR".to_string(), cache.to_string_lossy().into_owned())]);
    let snapshot = LspFixture::fixture_snapshot(workspace).with_env(env);
    let mut fixture = LspFixture::start_with_snapshot(workspace, snapshot);
    let folders: Vec<Value> = folders
        .iter()
        .map(|folder| json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "root" }))
        .collect();
    fixture.initialize(json!({
        "processId": null,
        "clientInfo": { "name": "Store Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    }));
    fixture
}

fn open(fixture: &mut LspFixture<'_>, path: &Path) {
    let text = std::fs::read_to_string(path).unwrap();
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(path), "languageId": "markdown", "version": 1, "text": text } }),
    );
}

fn result(fixture: &mut LspFixture<'_>, method: &str, params: Value) -> Value {
    let response = fixture.request(method, params);
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    response.result.unwrap()
}

/// `(path, zero-based line)` of each reference to the heading on line 0 of
/// `path`, sorted.
fn heading_references(fixture: &mut LspFixture<'_>, path: &Path) -> Vec<(String, u64)> {
    let answer = result(
        fixture,
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri(path) },
            "position": { "line": 0, "character": 3 },
            "context": { "includeDeclaration": false }
        }),
    );
    let mut out: Vec<(String, u64)> = answer
        .as_array()
        .unwrap()
        .iter()
        .map(|location| (location["uri"].as_str().unwrap().to_string(), location["range"]["start"]["line"].as_u64().unwrap()))
        .collect();
    out.sort();
    out
}

/// `uri -> [(start line, new text)]` of a workspace edit.
fn edits(answer: &Value) -> BTreeMap<String, Vec<(u64, String)>> {
    let mut out = BTreeMap::new();
    for (uri, edits) in answer["changes"].as_object().expect("a `changes` edit") {
        let mut list: Vec<(u64, String)> = edits
            .as_array()
            .unwrap()
            .iter()
            .map(|edit| (edit["range"]["start"]["line"].as_u64().unwrap(), edit["newText"].as_str().unwrap().to_string()))
            .collect();
        list.sort();
        out.insert(uri.clone(), list);
    }
    out
}

fn rename_heading(fixture: &mut LspFixture<'_>, path: &Path, new_name: &str) -> BTreeMap<String, Vec<(u64, String)>> {
    edits(&result(
        fixture,
        "textDocument/rename",
        json!({ "textDocument": { "uri": uri(path) }, "position": { "line": 0, "character": 3 }, "newName": new_name }),
    ))
}

fn rename_file(fixture: &mut LspFixture<'_>, from: &Path, to: &Path) -> BTreeMap<String, Vec<(u64, String)>> {
    edits(&result(fixture, "workspace/willRenameFiles", json!({ "files": [ { "oldUri": uri(from), "newUri": uri(to) } ] })))
}

/// The first session: fills every root's store, then shuts down.
fn fill(workspace: &LspWorkspace, folders: &[&Path], opened: &Path) {
    let mut fixture = start(workspace, folders);
    open(&mut fixture, opened);
    heading_references(&mut fixture, opened);
    fixture.shutdown();
}

/// Acceptance 3: after a restart, heading rename and file rename edit a
/// referrer that was never open and that no open document references.
#[test]
fn after_a_restart_renames_edit_a_referrer_never_open() {
    let workspace = LspWorkspace::new();
    let root = workspace.path().join("root");
    let target = root.join("target.md");
    write(&target, "# Target\n\nBody.\n");
    write(&root.join("deep/referrer.md"), "# Referrer\n\nSee [the target](../target.md#target).\n\nAnd [[target#Target]].\n");
    write(&root.join("plain.md"), "# Plain\n\n[path link](target.md)\n");
    fill(&workspace, &[&root], &target);

    let mut fixture = start(&workspace, &[&root]);
    open(&mut fixture, &target);
    let referrer = uri(&root.join("deep/referrer.md"));
    let plain = uri(&root.join("plain.md"));
    assert_eq!(heading_references(&mut fixture, &target), vec![(referrer.clone(), 2), (referrer.clone(), 4)]);

    let renamed = rename_heading(&mut fixture, &target, "Goal");
    assert_eq!(renamed[&referrer], vec![(2, "goal".to_string()), (4, "Goal".to_string())], "{renamed:?}");

    let moved = rename_file(&mut fixture, &target, &root.join("goal.md"));
    assert_eq!(moved.get(&plain), Some(&vec![(2, "goal.md".to_string())]), "{moved:?}");
    assert_eq!(moved[&referrer], vec![(4, "goal".to_string())], "the wiki link is re-spelled: {moved:?}");
    fixture.shutdown();
}

/// Acceptance 6: in a two-root workspace (two folders of one repository),
/// after a restart, a file rename in one root edits a path link and a wiki
/// link in the other.
///
/// The repository matters: a `../` link out of a folder that is in no
/// repository does not resolve, with or without the store.
#[test]
fn after_a_restart_a_file_rename_edits_a_referrer_in_the_other_root() {
    let workspace = LspWorkspace::new();
    init_repository(workspace.path());
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    let target = first.join("target.md");
    write(&target, "# Target\n");
    write(&second.join("referrer.md"), "# Referrer\n\n[across roots](../first/target.md)\n\n[[target]]\n\n[its heading](../first/target.md#target)\n");
    fill(&workspace, &[&first, &second], &target);

    let mut fixture = start(&workspace, &[&first, &second]);
    open(&mut fixture, &target);
    let referrer = uri(&second.join("referrer.md"));
    assert_eq!(heading_references(&mut fixture, &target), vec![(referrer.clone(), 6)]);
    let moved = rename_file(&mut fixture, &target, &first.join("moved.md"));
    assert_eq!(
        moved.get(&referrer),
        Some(&vec![(2, "../first/moved.md".to_string()), (4, "moved".to_string())]),
        "{moved:?}"
    );
    fixture.shutdown();
}

/// Acceptance 6, nested roots: a file under both roots has a row in each
/// store, yet references and workspace symbols list it once.
#[test]
fn with_nested_roots_no_result_appears_twice() {
    let workspace = LspWorkspace::new();
    let outer = workspace.path().join("outer");
    let inner = outer.join("inner");
    let target = inner.join("target.md");
    write(&target, "# Nested Target\n");
    write(&inner.join("referrer.md"), "# Inner Referrer\n\n[t](target.md#nested-target)\n");
    write(&outer.join("referrer.md"), "# Outer Referrer\n\n[t](inner/target.md#nested-target)\n");
    fill(&workspace, &[&outer, &inner], &target);

    let mut fixture = start(&workspace, &[&outer, &inner]);
    open(&mut fixture, &target);
    assert_eq!(
        heading_references(&mut fixture, &target),
        vec![(uri(&inner.join("referrer.md")), 2), (uri(&outer.join("referrer.md")), 2)]
    );
    let symbols = result(&mut fixture, "workspace/symbol", json!({ "query": "Inner Referrer" }));
    let names: Vec<&str> = symbols.as_array().unwrap().iter().map(|symbol| symbol["name"].as_str().unwrap()).collect();
    // The title symbol and the heading symbol, once each.
    assert_eq!(names, vec!["Inner Referrer", "Inner Referrer"], "{symbols:#}");
    fixture.shutdown();
}
