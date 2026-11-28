//! Host function call diagnostics through a real LSP session: DMLS reads the
//! function catalog and facts artifact of the registered host that
//! identifies the document and checks host calls exactly as composition
//! does. A bundle whose facts are missing, stale, incomplete, or refused
//! registers no host, so its functions are unknown, as in an unregistered
//! session.
//!
//! The catalog and artifact are the library's committed test host
//! (`darkmatter/lib/tests/fixtures/host_functions/`), installed as an
//! expression catalog of a format 3 bundle (the artifact is its facts
//! companion) registered in each session's home.
//! These tests cover the wiring only; the library's `host_functions` tests
//! walk the reader's whole input matrix.

use std::path::Path;

use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::Bundle;
use crate::union_item_editor::initialize_params;

const INCOMPATIBLE: &str = "dm.expression.incompatible_call";
const UNCERTAIN: &str = "dm.expression.uncertain_call";
const UNKNOWN_FUNCTION: &str = "dm.expression.unknown_function";

const CATALOG: &str = include_str!("../../../lib/tests/fixtures/host_functions/project.yaml");
const ARTIFACT: &str = include_str!("../../../lib/tests/fixtures/host_functions/project.facts.json");

/// Line 4 is a frontmatter span; lines 7–9 are the guarded, unguarded, and
/// category 5 body calls. Its `source` matches the bundle's trigger.
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

/// A matcher-only trigger: it identifies a document with `source`.
const TRIGGER: &str = "kind: schema-trigger\nmatch:\n    source: string(required)\n";

/// The `acme` bundle with its one trigger matching [`DOCUMENT`] and the
/// committed `project` catalog as its only expression catalog, with
/// `artifact` as its facts companion (missing when `None`, still digested).
fn bundle(catalog: &str, artifact: Option<&str>) -> Bundle {
    let mut bundle = Bundle::acme()
        .without_file("schemas/widget.yaml")
        .without_file("schemas/gadget.yaml")
        .without_file("schemas/functions.yaml")
        .with_file("schemas/project-trigger.yaml", TRIGGER)
        .with_file("schemas/project.yaml", catalog);
    bundle.members.remove("schemas/functions.facts.json");
    match artifact {
        Some(artifact) => bundle.with_member("schemas/project.facts.json", artifact),
        None => bundle,
    }
}

/// A strict-mode repository at `root` whose home registers `bundle`, if any.
fn workspace_with(root: &Path, bundle: Option<&Bundle>) {
    crate::registered_host_views::repository(root, &[]);
    if let Some(bundle) = bundle {
        bundle.write_to(&root.join("home/.project"));
        crate::registered_host_views::write(root, "home/.darkmatter.yaml", "register:\n  - ./.project/darkmatter-host.yaml\n");
    }
    std::fs::write(root.join(".dmls.toml"), "[expressions]\nstrict = true\n").unwrap();
}

/// The published diagnostics of [`DOCUMENT`] in a session registering
/// `bundle`, if any.
fn diagnostics(bundle: Option<&Bundle>) -> Vec<Value> {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    workspace_with(root, bundle);
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
    // The buffered take returns the latest publication first; the
    // earlier ones still buffered are dropped.
    let latest = fixture.wait_for_diagnostics_params(&uri);
    while fixture.take_buffered_diagnostics(&uri).is_some() {}
    fixture.shutdown();
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// The call and unknown-function findings as sorted `(line, code)` pairs.
fn findings(diagnostics: &[Value]) -> Vec<(u64, String)> {
    let mut found: Vec<(u64, String)> = diagnostics
        .iter()
        .filter_map(|diagnostic| {
            let code = diagnostic["code"].as_str()?;
            [INCOMPATIBLE, UNCERTAIN, UNKNOWN_FUNCTION]
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

/// The registered host's trusted facts: the guarded call is silent, the
/// unguarded one warns, and `project_label(42)` is incompatible in body and
/// frontmatter.
fn complete() -> Vec<(u64, String)> {
    expect(&[(4, INCOMPATIBLE), (8, UNCERTAIN), (9, INCOMPATIBLE)])
}

/// No host: what an unregistered session reports, where both functions
/// are unknown.
fn unregistered() -> Vec<(u64, String)> {
    let found = findings(&diagnostics(None));
    assert!(found.iter().any(|(_, code)| code == UNKNOWN_FUNCTION), "{found:#?}");
    found
}

/// Criteria 17 and 37, editor side: with the committed artifact, host calls
/// are checked like built-ins.
#[test]
fn the_host_catalog_and_its_artifact_check_host_calls_like_built_ins() {
    let found = diagnostics(Some(&bundle(CATALOG, Some(ARTIFACT))));
    assert_eq!(findings(&found), complete(), "{found:#?}");
}

/// A bundle whose facts artifact is missing, records another catalog
/// version, lacks a function, or disagrees with the catalog's
/// `narrows(...)` fact registers no host: its functions are unknown, and
/// no stale verdict is kept.
#[test]
fn untrusted_host_facts_register_no_host() {
    let edited = CATALOG.replace("the project's label", "the project's edited label");
    assert_ne!(edited, CATALOG);
    let edit = |edit: fn(&mut Value)| {
        let mut root: Value = serde_json::from_str(ARTIFACT).unwrap();
        edit(&mut root);
        serde_json::to_string_pretty(&root).unwrap()
    };
    let lacking = edit(|root| {
        root["functions"].as_object_mut().unwrap().remove("project_label");
    });
    let no_facts = edit(|root| root["functions"]["is_project_file"]["facts"] = json!([]));
    let wrong_kind = edit(|root| root["functions"]["is_project_file"]["facts"][0]["kind"] = json!("true_iff"));
    let expected = unregistered();
    for (label, bundle) in [
        ("missing", bundle(CATALOG, None)),
        ("another catalog version", bundle(&edited, Some(ARTIFACT))),
        ("lacking a function", bundle(CATALOG, Some(&lacking))),
        ("facts: []", bundle(CATALOG, Some(&no_facts))),
        ("wrong kind", bundle(CATALOG, Some(&wrong_kind))),
    ] {
        let found = diagnostics(Some(&bundle));
        assert_eq!(findings(&found), expected, "{label}: {found:#?}");
    }
}

/// A function catalog that does not register (here: it redeclares a
/// built-in) refuses the host, so its functions stay unknown.
#[test]
fn a_host_function_catalog_that_fails_to_load_registers_no_host() {
    let catalog = "kind: expression-schema\ncategories:\n  - Math\ntypes:\n  min: |-\n    function(parameters(a: number); returns(number); category(Math))\n";
    let found = diagnostics(Some(&bundle(catalog, Some(ARTIFACT))));
    assert_eq!(findings(&found), unregistered(), "{found:#?}");
}

#[test]
fn a_running_editor_recovers_and_withdraws_host_facts_after_watched_changes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let valid = bundle(CATALOG, Some(ARTIFACT));
    workspace_with(root, Some(&valid));
    let install = root.join("home/.project");
    let manifest = install.join("darkmatter-host.yaml");
    let catalog_path = install.join("schemas/project.yaml");
    let artifact_path = install.join("schemas/project.facts.json");
    std::fs::remove_file(&artifact_path).unwrap();
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
    let changed = |fixture: &LspFixture<'_>, paths: &[&Path], kind: u8| {
        let changes: Vec<Value> = paths
            .iter()
            .map(|path| json!({ "uri": url::Url::from_file_path(path).unwrap().as_str(), "type": kind }))
            .collect();
        fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
    };
    let has_unknown = |found: &[(u64, String)]| found.iter().any(|(_, code)| code == UNKNOWN_FUNCTION);
    let found = findings(&latest(&mut fixture));
    assert!(has_unknown(&found), "the missing facts refuse the host: {found:#?}");

    std::fs::write(&artifact_path, ARTIFACT).unwrap();
    changed(&fixture, &[&artifact_path], 1);
    assert_eq!(findings(&latest(&mut fixture)), complete(), "created artifact restores narrowing");

    // A republished catalog its facts were not generated for.
    let edited = CATALOG.replace("the project's label", "the project's edited label");
    assert_ne!(edited, CATALOG);
    bundle(&edited, Some(ARTIFACT)).write_to(&install);
    changed(&fixture, &[&catalog_path, &manifest], 2);
    let found = findings(&latest(&mut fixture));
    assert!(has_unknown(&found), "stale facts cannot keep a verdict: {found:#?}");

    valid.write_to(&install);
    changed(&fixture, &[&catalog_path, &manifest], 2);
    assert_eq!(findings(&latest(&mut fixture)), complete(), "matching catalog restores narrowing");

    std::fs::remove_file(&artifact_path).unwrap();
    changed(&fixture, &[&artifact_path], 3);
    let found = findings(&latest(&mut fixture));
    assert!(has_unknown(&found), "deleted facts withdraw narrowing: {found:#?}");
    fixture.shutdown();
}
