//! Two matched triggers of one registered host that disagree on a property
//! are one `dm.host.trigger_conflict` error on the document in the editor,
//! related to both installed trigger files, and a bundle that makes them
//! agree clears it.

use serde_json::Value;

use crate::common::LspWorkspace;
use crate::host_fixtures::{Bundle, RUN_NAMESPACE, lifecycle_artifact_for};
use crate::host_registration_watches::start;
use crate::registered_host_views::repository;
use crate::union_item_editor::open_text;

/// A trigger matching every document with a `team` string, declaring
/// `agent` as `agent`.
fn trigger(agent: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    agent: {agent}\n")
}

fn bundle(second: &str) -> Bundle {
    let mut bundle =
        Bundle::acme().with_file("schemas/a.yaml", &trigger("number")).with_file("schemas/b.yaml", &trigger(second));
    bundle
        .members
        .insert("schemas/globals.lifecycle.json".into(), lifecycle_artifact_for("acme", &[("schemas/run.yaml", RUN_NAMESPACE)]));
    bundle
}

fn conflicts(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == "dm.host.trigger_conflict").collect()
}

#[test]
fn a_conflict_between_matched_triggers_is_one_error_related_to_both_files() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    bundle("boolean").write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.md", "---\nteam: core\nagent: 3\n---\n# Team\n");
    let [conflict] = conflicts(&diagnostics)[..] else { panic!("one conflict: {diagnostics:#?}") };
    assert_eq!(conflict["severity"], 1, "{conflict:#?}");
    let related: Vec<&str> = conflict["relatedInformation"]
        .as_array()
        .expect("related triggers")
        .iter()
        .filter_map(|information| information["location"]["uri"].as_str())
        .collect();
    assert_eq!(related.len(), 2, "{conflict:#?}");
    assert!(related[0].ends_with("home/.acme/schemas/a.yaml"), "{conflict:#?}");
    assert!(related[1].ends_with("home/.acme/schemas/b.yaml"), "{conflict:#?}");
    fixture.shutdown();

    // The same document under agreeing triggers has no conflict.
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    bundle("number").write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.md", "---\nteam: core\nagent: 3\n---\n# Team\n");
    assert!(conflicts(&diagnostics).is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}
