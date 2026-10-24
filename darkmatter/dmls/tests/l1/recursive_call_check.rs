//! The design-time call check on recursive-typed arguments, as DMLS reports it.
//!
//! A property typed by a recursive named type reaches the editor's call check
//! as a named reference. DMLS resolves it through the document's effective
//! definition table, so it reports exactly the calls composition rejects:
//! passing the recursive object to a number parameter is
//! `dm.expression.incompatible_call`, passing it to an object parameter is
//! fine, and both documents are checked without expanding the type.

use crate::common;

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, RequestSnapshot};
use serde_json::{Value, json};

const TYPES: &str = "kind: schema
types:
  node: \"{ label: string(required), children: node[](required) }\"
";

const FRONTMATTER: &str = "$schema:
  tree: node(required)@./types.yaml
tree:
  label: root
  children:
    - label: a
      children: []
";

fn document(body: &str) -> String {
    format!("---\n{FRONTMATTER}---\n\n{body}\n")
}

fn composition_rejects(workspace: &LspWorkspace, path: &std::path::Path) -> bool {
    let dir = workspace.path();
    let options =
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(path.to_path_buf());
    let snapshot = RequestSnapshot::new(dir).with_home(Some(dir.join("home")));
    let request = ComposeRequest::prepare(options, &snapshot).expect("request");
    match Markdown::try_from(path).expect("document loads").compose_with(&request) {
        Ok(_) => false,
        Err(error) => error.rejected_call().is_some(),
    }
}

/// The `dm.expression.*_call` diagnostics published for `text`, opened as
/// `name.md` in `workspace`.
fn call_diagnostics(fixture: &mut LspFixture<'_>, workspace: &LspWorkspace, name: &str, text: &str) -> Vec<Value> {
    let path = workspace.path().join(format!("{name}.md"));
    std::fs::write(&path, text).unwrap();
    let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
    );
    fixture.flush_server();
    let mut latest = fixture.wait_for_diagnostics_params(&uri);
    while let Some(params) = fixture.take_buffered_diagnostics(&uri) {
        latest = params;
    }
    latest["diagnostics"]
        .as_array()
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.ends_with("_call")))
        .cloned()
        .collect()
}

/// Without the definition table both calls are category 4: the number call
/// would show nothing and compose until `min` failed at evaluation.
#[test]
fn the_editor_reports_exactly_the_recursive_call_composition_rejects() {
    let started = std::time::Instant::now();
    let workspace = LspWorkspace::new();
    std::fs::write(workspace.path().join("types.yaml"), TYPES).unwrap();
    let root_uri = url::Url::from_directory_path(workspace.path()).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "recursive" } ]
    }));

    let incompatible = document("{{ min(tree, 5) }}");
    let found = call_diagnostics(&mut fixture, &workspace, "incompatible", &incompatible);
    let compatible = document("{{ keys(tree) }}");
    let clean = call_diagnostics(&mut fixture, &workspace, "compatible", &compatible);
    fixture.shutdown();

    assert!(
        found.iter().any(|diagnostic| diagnostic["code"] == json!("dm.expression.incompatible_call")
            && diagnostic["severity"] == json!(1)
            && diagnostic["message"].as_str().is_some_and(|message| message.contains("is node, expected"))),
        "{found:#?}"
    );
    assert!(clean.is_empty(), "{clean:#?}");

    assert!(composition_rejects(&workspace, &workspace.path().join("incompatible.md")));
    assert!(!composition_rejects(&workspace, &workspace.path().join("compatible.md")));
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "took {:?}", started.elapsed());
}
