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

/// Two named triggers' conflict names each by its label beside its path;
/// both files stay related.
#[test]
fn a_conflict_between_named_triggers_names_both_labels() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let mut named = bundle("boolean");
    for (path, name) in [("schemas/a.yaml", "alpha-schema"), ("schemas/b.yaml", "beta-schema")] {
        let text = named.members[path].clone();
        named = named.with_member(path, &format!("name: {name}\n{text}"));
    }
    named.write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.md", "---\nteam: core\nagent: 3\n---\n# Team\n");
    let [conflict] = conflicts(&diagnostics)[..] else { panic!("one conflict: {diagnostics:#?}") };
    let message = conflict["message"].as_str().unwrap_or_default().replace('\\', "/");
    assert!(message.contains("`alpha-schema (") && message.contains("home/.acme/schemas/a.yaml)`"), "{conflict:#?}");
    assert!(message.contains("`beta-schema (") && message.contains("home/.acme/schemas/b.yaml)`"), "{conflict:#?}");
    assert_eq!(conflict["relatedInformation"].as_array().map(Vec::len), Some(2), "{conflict:#?}");
    fixture.shutdown();
}

/// A narrowing whose alternatives carry each other's description, against
/// the key every arm of a root union shares, is the same conflict, and the
/// schema's own judgment of the value is withheld with it.
#[test]
fn a_narrowing_that_swaps_descriptions_between_alternatives_is_a_conflict() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let wide = "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    review-model:\n        - \"string -> single model\"\n        - \"string[] -> model choices\"\n";
    let key = "      review-model:\n          - \"enum(claude, codex) -> model choices\"\n          - \"string[] -> single model\"\n";
    let union = format!(
        "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    - prompt: string(required)\n      sections: null\n{key}    - sections: object(required)\n      prompt: null\n{key}"
    );
    let swapped = bundle("number").with_member("schemas/a.yaml", wide).with_member("schemas/b.yaml", &union);
    swapped.write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (_, diagnostics) =
        open_text(&mut fixture, &workspace, "notes/team.md", "---\nteam: core\nprompt: Say hello\nreview-model: gemini\n---\n# Team\n");
    let [conflict] = conflicts(&diagnostics)[..] else { panic!("one conflict: {diagnostics:#?}") };
    assert!(conflict["message"].as_str().is_some_and(|message| message.contains("`review-model`")), "{conflict:#?}");
    assert_eq!(diagnostics.len(), 1, "the conflict alone, no judgment of `review-model`: {diagnostics:#?}");
    fixture.shutdown();
}

/// A list narrowed by a key every arm of a root union shares (a string
/// list under a list of strings or numbers) is no conflict: a fitting
/// document has no diagnostic, and the narrowed list still judges an item.
#[test]
fn a_list_narrowing_shared_by_a_root_union_publishes_no_conflict() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let wide = "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    review-model: \"(string | number)[]\"\n";
    let key = "      review-model: \"string[]\"\n";
    let union = format!(
        "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    - prompt: string(required)\n      sections: null\n{key}    - sections: object(required)\n      prompt: null\n{key}"
    );
    bundle("number").with_member("schemas/a.yaml", wide).with_member("schemas/b.yaml", &union).write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (_, diagnostics) =
        open_text(&mut fixture, &workspace, "notes/team.md", "---\nteam: core\nprompt: Say hello\nreview-model: [claude]\n---\n# Team\n");
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let (_, diagnostics) = open_text(
        &mut fixture,
        &workspace,
        "notes/bad.md",
        "---\nteam: core\nprompt: Say hello\nreview-model: [{ name: claude }]\n---\n# Team\n",
    );
    assert!(conflicts(&diagnostics).is_empty(), "{diagnostics:#?}");
    assert!(!diagnostics.is_empty(), "the narrowed list judges the item: {diagnostics:#?}");
    fixture.shutdown();
}

/// A described stored list narrowed by the same list written inline
/// (`review-model`), and a stored list beside its exact inline body
/// (`equivalent`), publish no conflict, and the layer still judges an item.
#[test]
fn a_stored_list_compared_with_its_inline_spelling_publishes_no_conflict() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let head = "kind: schema-trigger\nmatch:\n    team: string(required)\n";
    let stored = format!(
        "{head}types:\n    Models: \"(string | {{ id: string }})[]\"\n$schema:\n    review-model: \"Models -> Models reviewed\"\n    equivalent: Models\n"
    );
    let inline = format!(
        "{head}$schema:\n    review-model: \"{{ id: string }}[] -> Models reviewed\"\n    equivalent: \"(string | {{ id: string }})[]\"\n"
    );
    bundle("number").with_member("schemas/a.yaml", &stored).with_member("schemas/b.yaml", &inline).write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (_, diagnostics) = open_text(
        &mut fixture,
        &workspace,
        "notes/team.md",
        "---\nteam: core\nprompt: Say hello\nreview-model: [{ id: claude }]\nequivalent: [claude]\n---\n# Team\n",
    );
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    for (file, bad) in [("notes/bad-model.md", "review-model: [claude]"), ("notes/bad-equivalent.md", "equivalent: [{ id: [1] }]")] {
        let (_, diagnostics) =
            open_text(&mut fixture, &workspace, file, &format!("---\nteam: core\nprompt: Say hello\n{bad}\n---\n# Team\n"));
        assert!(conflicts(&diagnostics).is_empty(), "{bad}: {diagnostics:#?}");
        assert!(!diagnostics.is_empty(), "the layer judges {bad}: {diagnostics:#?}");
    }
    fixture.shutdown();
}

/// A recursive named tree and the same tree written as one inline
/// unfolding publish no conflict, and the layer still judges a child.
#[test]
fn a_recursive_reference_and_its_inline_unfolding_publish_no_conflict() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let trigger = |ty: &str| {
        format!(
            "kind: schema-trigger\nmatch:\n    team: string(required)\ntypes:\n    Node: \"{{ name: string, children: Node[] }}\"\n$schema:\n    review-model: \"{ty}\"\n"
        )
    };
    bundle("number")
        .with_member("schemas/a.yaml", &trigger("Node"))
        .with_member("schemas/b.yaml", &trigger("{ name: string, children: Node[] }"))
        .write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let tree = |leaf: &str| format!("---\nteam: core\nprompt: Say hello\nreview-model: {{ name: root, children: [{{ name: {leaf} }}] }}\n---\n# Team\n");
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.md", &tree("leaf"));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/bad.md", &tree("[1]"));
    assert!(conflicts(&diagnostics).is_empty(), "{diagnostics:#?}");
    assert!(!diagnostics.is_empty(), "the layer judges the child: {diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn a_narrower_registered_list_item_type_is_used_by_the_editor() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let trigger = |ty: &str| format!("kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    review-model: \"{ty}\"\n");
    bundle("number")
        .with_member("schemas/a.yaml", &trigger("(string | number)[]"))
        .with_member("schemas/b.yaml", &trigger("enum(claude, codex)[]"))
        .write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let document = |model: &str| format!("---\nteam: core\nreview-model: [{model}]\n---\n# Team\n");
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.md", &document("claude"));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/bad.md", &document("gemini"));
    assert!(conflicts(&diagnostics).is_empty(), "{diagnostics:#?}");
    assert!(!diagnostics.is_empty(), "the winning list checks the item");
    fixture.shutdown();
}
