//! The shared review/fix cycle schema (`schemas/review-fix-cycle.yaml`)
//! refuses an existing file outside `spec`'s `match(**/*spec*.md)` in the
//! editor, as it does in `md` and the library validator.
//!
//! Boundary: a real LSP session (`LspFixture`) over a temporary workspace
//! holding a copy of the schema.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const CONSTRAINT: &str = "dm.schema.constraint";

#[test]
fn an_existing_plan_in_spec_is_a_constraint_diagnostic_and_the_spec_is_clean() {
    let workspace = LspWorkspace::new();
    let dir = workspace.path();
    std::fs::write(
        dir.join("review-fix-cycle.yaml"),
        include_str!("../../../../schemas/review-fix-cycle.yaml"),
    )
    .unwrap();
    std::fs::write(dir.join("spec.md"), "# Spec\n").unwrap();
    std::fs::write(dir.join("plan.md"), "# Plan\n").unwrap();
    let root_uri = url::Url::from_directory_path(dir).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(json!({
        "processId": null,
        "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
        "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "review" } ]
    }));

    let mut constraint_lines = |name: &str, spec: &str| -> Vec<u64> {
        let text = format!("---\n$schema: ./review-fix-cycle.yaml\nspec: {spec}\n---\n# Review\n");
        let path = dir.join(name);
        std::fs::write(&path, &text).unwrap();
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
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter(|diagnostic| diagnostic["code"] == CONSTRAINT)
            .map(|diagnostic: &Value| diagnostic["range"]["start"]["line"].as_u64().unwrap())
            .collect()
    };

    assert_eq!(constraint_lines("valid.md", "./spec.md"), Vec::<u64>::new());
    assert_eq!(constraint_lines("wrong.md", "./plan.md"), vec![2], "the `spec:` line is flagged");
    fixture.shutdown();
}
