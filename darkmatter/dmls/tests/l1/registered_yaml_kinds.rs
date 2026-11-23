//! A whole YAML document whose `kind` only a registered host defines is
//! judged in the editor by that host's `kind: yaml-schema` definition, under
//! the installed registrations (no Markdown trigger work needed); two
//! different registered definitions of the kind are one error on the `kind`
//! value whose related locations are the installed definition files; and a
//! registration change re-prepares the document's contract without a
//! restart.

use serde_json::Value;

use crate::common::LspWorkspace;
use crate::host_fixtures::{Bundle, MANIFEST_NAME, RUN_NAMESPACE, lifecycle_artifact_for};
use crate::host_registration_watches::{CHANGED, report, start};
use crate::registered_host_views::{repository, write};
use crate::union_item_editor::open_text;

/// A registered definition of `kind: roster`.
pub(crate) const ROSTER: &str = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: string[](required)\n";

pub(crate) const TEAM: &str = "kind: roster\nplatforms: 3\n";

/// `acme`, named `host`, with `roster` written as `definition`.
pub(crate) fn bundle(host: &str, definition: &str) -> Bundle {
    let mut bundle = Bundle::acme_named(host).with_file("schemas/roster.yaml", definition);
    bundle.members.insert(
        "schemas/globals.lifecycle.json".into(),
        lifecycle_artifact_for(host, &[("schemas/run.yaml", RUN_NAMESPACE)]),
    );
    bundle
}

pub(crate) fn errors(diagnostics: &[Value]) -> Vec<&Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["severity"] == 1).collect()
}

#[test]
fn a_registered_kind_judges_the_document_and_a_competing_definition_refuses_it_until_removed() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    bundle("acme", ROSTER).write_to(&root.join("home/.acme"));
    bundle("beta", &ROSTER.replace("string[]", "number[]")).write_to(&root.join("home/.beta"));
    let mut fixture = start(&workspace);

    // The registered definition judges `platforms: 3`, at the value.
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/team.yaml", TEAM);
    let found = errors(&diagnostics);
    assert_eq!(found.len(), 1, "{diagnostics:#?}");
    assert_eq!(found[0]["range"]["start"]["line"], 1, "{diagnostics:#?}");
    assert_eq!(found[0]["range"]["start"]["character"], 11, "{diagnostics:#?}");

    // A second host defining `roster` differently: one error on the `kind`
    // value, related to each installed definition.
    let config = root.join("home/.darkmatter.yaml");
    write(root, "home/.darkmatter.yaml", &format!("register:\n  - ./.acme/{MANIFEST_NAME}\n  - ./.beta/{MANIFEST_NAME}\n"));
    report(&mut fixture, &document, &[(&config, CHANGED)]);
    let diagnostics = fixture.take_latest_diagnostics_ending("notes/team.yaml").expect("republished");
    let [duplicate] = errors(&diagnostics)[..] else { panic!("one error: {diagnostics:#?}") };
    assert_eq!(duplicate["range"]["start"]["line"], 0, "on the kind: {duplicate:#?}");
    let related: Vec<&str> = duplicate["relatedInformation"]
        .as_array()
        .expect("related definitions")
        .iter()
        .filter_map(|information| information["location"]["uri"].as_str())
        .collect();
    assert!(related.iter().any(|uri| uri.ends_with("home/.acme/schemas/roster.yaml")), "{duplicate:#?}");
    assert!(related.iter().any(|uri| uri.ends_with("home/.beta/schemas/roster.yaml")), "{duplicate:#?}");

    // Removing the second registration restores the first definition.
    write(root, "home/.darkmatter.yaml", &format!("register:\n  - ./.acme/{MANIFEST_NAME}\n"));
    report(&mut fixture, &document, &[(&config, CHANGED)]);
    let diagnostics = fixture.take_latest_diagnostics_ending("notes/team.yaml").expect("republished");
    let found = errors(&diagnostics);
    assert_eq!(found.len(), 1, "{diagnostics:#?}");
    assert_eq!(found[0]["range"]["start"]["line"], 1, "{diagnostics:#?}");
    fixture.shutdown();
}
