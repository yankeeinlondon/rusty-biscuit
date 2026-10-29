//! Host function call diagnostics through a real LSP session: DMLS reads a
//! host function catalog and its `<stem>.facts.json` from the document's
//! schema roots and checks host calls exactly as composition does, or
//! reports incomplete analysis when the artifact cannot be trusted.
//!
//! The catalog and artifact are the library's committed test host
//! (`darkmatter/lib/tests/fixtures/host_functions/`), copied into each
//! workspace's `schemas/` root. These tests cover the wiring only; the
//! library's `host_functions` tests walk the reader's whole input matrix.

use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::union_item_editor::initialize_params;

const INCOMPLETE: &str = "dm.expression.incomplete_analysis";
const INCOMPATIBLE: &str = "dm.expression.incompatible_call";
const UNCERTAIN: &str = "dm.expression.uncertain_call";
const HOST_CATALOG: &str = "dm.schema.host_catalog";
const UNKNOWN_FUNCTION: &str = "dm.expression.unknown_function";

/// Line 4 is a frontmatter span; lines 7–9 are the guarded, unguarded, and
/// category 5 body calls.
const DOCUMENT: &str = "---
$schema:
  source: unknown
source: note.md
label: \"{{ project_label(42) }}\"
---

{{ is_project_file(source) && frontmatter(source, 'title') }}
{{ frontmatter(source, 'title') }}
{{ project_label(42) }}
";

/// A strict-mode repository whose `schemas/` root holds `catalog` and, when
/// given, `artifact` beside it; returns the published diagnostics of
/// [`DOCUMENT`].
fn diagnostics(catalog: &str, artifact: Option<&str>) -> Vec<Value> {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    for dir in [".git/objects", ".git/refs/heads", "schemas"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(root.join(".dmls.toml"), "[expressions]\nstrict = true\n").unwrap();
    std::fs::write(root.join("schemas/project.yaml"), catalog).unwrap();
    if let Some(artifact) = artifact {
        std::fs::write(root.join("schemas/project.facts.json"), artifact).unwrap();
    }
    let path = root.join("doc.md");
    std::fs::write(&path, DOCUMENT).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": DOCUMENT } }),
    );
    fixture.flush_server();
    let mut latest = fixture.wait_for_diagnostics_params(&uri);
    while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
        latest = params;
    }
    fixture.shutdown();
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// The call, incomplete-analysis, host-catalog, and unknown-function
/// findings as sorted `(line, code)` pairs.
fn findings(diagnostics: &[Value]) -> Vec<(u64, String)> {
    let mut found: Vec<(u64, String)> = diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let code = diagnostic["code"].as_str()?;
            [INCOMPLETE, INCOMPATIBLE, UNCERTAIN, HOST_CATALOG, UNKNOWN_FUNCTION]
                .contains(&code)
                .then(|| (diagnostic["range"]["start"]["line"].as_u64().unwrap(), code.to_string()))
        })
        .collect();
    found.sort();
    found
}

fn expect(rows: &[(u64, &str)]) -> Vec<(u64, String)> {
    rows.iter().map(|(line, code)| (*line, (*code).to_string())).collect()
}

/// The incomplete-analysis result when `is_project_file` and
/// `project_label` both lack trusted facts: each call warns, the guard no
/// longer narrows, and `project_label(42)` has no category.
fn both_incomplete() -> Vec<(u64, String)> {
    expect(&[(4, INCOMPLETE), (7, INCOMPLETE), (7, UNCERTAIN), (8, UNCERTAIN), (9, INCOMPLETE)])
}

/// Criteria 17 and 37, editor side: with the committed artifact, the guarded
/// call is silent, the unguarded one warns, `project_label(42)` is
/// incompatible in body and frontmatter, and nothing is incomplete.
#[test]
fn the_host_catalog_and_its_artifact_check_host_calls_like_built_ins() {
    let catalog = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
    let artifact = include_str!("../../../lib/tests/fixtures/host_functions/project.facts.json");
    let found = diagnostics(catalog, Some(artifact));
    assert_eq!(findings(&found), expect(&[(4, INCOMPATIBLE), (8, UNCERTAIN), (9, INCOMPATIBLE)]), "{found:#?}");
}

#[test]
fn a_missing_host_facts_artifact_reports_incomplete_analysis() {
    let catalog = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
    let found = diagnostics(catalog, None);
    assert_eq!(findings(&found), both_incomplete(), "{found:#?}");
    let message = found.iter().find(|diagnostic| diagnostic["code"] == json!(INCOMPLETE)).unwrap()["message"].as_str().unwrap();
    assert!(message.contains("is absent"), "{message}");
}

#[test]
fn a_host_facts_artifact_for_another_catalog_version_reports_incomplete_analysis() {
    let catalog = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
    let artifact = include_str!("../../../lib/tests/fixtures/host_functions/project.facts.json");
    let edited = catalog.replace("the project's label", "the project's edited label");
    assert_ne!(edited, catalog);
    let found = diagnostics(&edited, Some(artifact));
    assert_eq!(findings(&found), both_incomplete(), "{found:#?}");
    let message = found.iter().find(|diagnostic| diagnostic["code"] == json!(INCOMPLETE)).unwrap()["message"].as_str().unwrap();
    assert!(message.contains("regenerate the artifact"), "{message}");
}

/// Only the function the artifact lacks is incomplete; the other keeps its
/// narrowing.
#[test]
fn a_host_facts_artifact_lacking_a_function_reports_it_incomplete() {
    let catalog = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
    let artifact = include_str!("../../../lib/tests/fixtures/host_functions/project.facts.json");
    let mut root: Value = serde_json::from_str(artifact).unwrap();
    root["functions"].as_object_mut().unwrap().remove("project_label");
    let found = diagnostics(catalog, Some(&serde_json::to_string_pretty(&root).unwrap()));
    assert_eq!(findings(&found), expect(&[(4, INCOMPLETE), (8, UNCERTAIN), (9, INCOMPLETE)]), "{found:#?}");
}

/// A function catalog that does not register (here: it redeclares a
/// built-in) is reported like a host global catalog that fails to load, and
/// its functions stay unknown.
#[test]
fn a_host_function_catalog_that_fails_to_load_is_reported() {
    let catalog = "kind: schema\ncategories:\n  - Math\ntypes:\n  min: |-\n    function(parameters(a: number); returns(number); category(Math))\n";
    let found = diagnostics(catalog, None);
    let messages: Vec<&str> = found
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(HOST_CATALOG))
        .filter_map(|diagnostic| diagnostic["message"].as_str())
        .collect();
    assert_eq!(messages.len(), 1, "{found:#?}");
    assert!(messages[0].contains("project.yaml") && messages[0].contains("built-in"), "{}", messages[0]);
}

#[test]
fn a_running_editor_recovers_and_withdraws_host_facts_after_watched_changes() {
    let catalog = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
    let artifact = include_str!("../../../lib/tests/fixtures/host_functions/project.facts.json");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    for directory in [".git/objects", ".git/refs/heads", "schemas"] {
        std::fs::create_dir_all(root.join(directory)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(root.join(".dmls.toml"), "[expressions]\nstrict = true\n").unwrap();
    let catalog_path = root.join("schemas/project.yaml");
    let artifact_path = root.join("schemas/project.facts.json");
    std::fs::write(&catalog_path, catalog).unwrap();
    let document_path = root.join("doc.md");
    std::fs::write(&document_path, DOCUMENT).unwrap();
    let uri = url::Url::from_file_path(&document_path).unwrap().to_string();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(initialize_params(root));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": DOCUMENT } }),
    );
    let latest = |fixture: &mut LspFixture<'_>| {
        fixture.flush_server();
        let mut publication = fixture.wait_for_diagnostics_params(&uri);
        while let Some(next) = fixture.take_buffered_diagnostics(&uri) {
            publication = next;
        }
        publication["diagnostics"].as_array().unwrap().clone()
    };
    let changed = |fixture: &LspFixture<'_>, path: &std::path::Path, kind: u8| {
        let changed_uri = url::Url::from_file_path(path).unwrap();
        fixture.notify(
            "workspace/didChangeWatchedFiles",
            json!({ "changes": [{ "uri": changed_uri.as_str(), "type": kind }] }),
        );
    };
    let complete = expect(&[(4, INCOMPATIBLE), (8, UNCERTAIN), (9, INCOMPATIBLE)]);
    assert_eq!(findings(&latest(&mut fixture)), both_incomplete());

    std::fs::write(&artifact_path, artifact).unwrap();
    changed(&fixture, &artifact_path, 1);
    assert_eq!(findings(&latest(&mut fixture)), complete, "created artifact restores narrowing");

    let edited = catalog.replace("the project's label", "the project's edited label");
    assert_ne!(edited, catalog);
    std::fs::write(&catalog_path, edited).unwrap();
    changed(&fixture, &catalog_path, 2);
    assert_eq!(findings(&latest(&mut fixture)), both_incomplete(), "stale facts cannot keep a verdict");

    std::fs::write(&catalog_path, catalog).unwrap();
    changed(&fixture, &catalog_path, 2);
    assert_eq!(findings(&latest(&mut fixture)), complete, "matching catalog restores narrowing");

    std::fs::remove_file(&artifact_path).unwrap();
    changed(&fixture, &artifact_path, 3);
    assert_eq!(findings(&latest(&mut fixture)), both_incomplete(), "deleted facts withdraw narrowing");
    fixture.shutdown();
}
