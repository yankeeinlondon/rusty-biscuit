//! A registered host's global catalog, prepared from its
//! `kind: namespace-schema` files (`config::registration::namespaces`), as
//! `load_entries` publishes it: one global per namespace, the digest map its
//! `catalog_hash` covers (transitive imports included), identity independent
//! of the install folder, declaration positions, the namespace envelope and
//! name rules, and the lifecycle artifact's binding to the catalog. Each case
//! makes one edit to the valid `acme` bundle.

use crate::host_fixtures::*;
use darkmatter::config::registration::{HOST_REGISTRATION_CODE, LoadedRegistrations, RegistrationDiagnostic};

const LIFECYCLE: &str = "schemas/globals.lifecycle.json";

/// A second namespace, `loop`.
const LOOP_NAMESPACE: &str = "\
kind: namespace-schema
namespace: loop
$schema:
    index: \"number -> The iteration.\"
";

/// The `acme` bundle with its `run` namespace replaced by `text` and the
/// artifact regenerated for it (which `text` must not import into).
fn with_run(text: &str) -> Bundle {
    Bundle::acme()
        .with_member("schemas/run.yaml", text)
        .with_member(LIFECYCLE, &lifecycle_artifact_for("acme", &[("schemas/run.yaml", text)]))
}

fn load(bundle: &Bundle) -> (Fixture, LoadedRegistrations) {
    let fixture = Fixture::new();
    fixture.install_bundle("host", bundle);
    let loaded = fixture.load();
    (fixture, loaded)
}

/// The one refusal of a load that published nothing.
fn refused(loaded: &LoadedRegistrations) -> &RegistrationDiagnostic {
    assert!(loaded.profiles.is_empty(), "nothing is published: {:?}", loaded.profiles);
    let [diagnostic] = loaded.diagnostics.as_slice() else { panic!("one refusal: {:#?}", loaded.diagnostics) };
    assert_eq!(diagnostic.code, HOST_REGISTRATION_CODE);
    diagnostic
}

/// The one refusal, in the `run` namespace file (`files[3]`).
fn refused_in_run<'a>(fixture: &Fixture, loaded: &'a LoadedRegistrations) -> &'a RegistrationDiagnostic {
    let diagnostic = refused(loaded);
    assert_eq!(diagnostic.file, fixture.path("host/schemas/run.yaml"), "{diagnostic:#?}");
    assert_eq!(diagnostic.member.as_deref(), Some("files[3]"), "{diagnostic:#?}");
    diagnostic
}

// ── the catalog ─────────────────────────────────────────────────────────────

#[test]
fn a_multi_file_catalog_declares_one_global_per_namespace_in_bundle_path_order() {
    let namespaces = [("schemas/loop.yaml", LOOP_NAMESPACE), ("schemas/run.yaml", RUN_NAMESPACE)];
    let bundle = Bundle::acme()
        .with_file("schemas/loop.yaml", LOOP_NAMESPACE)
        .with_member(LIFECYCLE, &lifecycle_artifact_for("acme", &namespaces));

    let (fixture, loaded) = load(&bundle);

    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let profile = &loaded.profiles[0];
    let declared: Vec<(&str, &str)> =
        profile.namespaces().iter().map(|member| (member.namespace.as_str(), member.path.as_str())).collect();
    assert_eq!(declared, [("loop", "schemas/loop.yaml"), ("run", "schemas/run.yaml")]);
    let globals: Vec<&str> = profile.globals().globals().map(|global| global.name()).collect();
    assert_eq!(globals, ["loop", "run"]);
    assert_eq!(profile.catalog_hash(), catalog_hash_of(&namespaces));
    assert_eq!(profile.namespace("loop").map(|member| member.location.clone()), Some(fixture.path("host/schemas/loop.yaml")));
}

#[test]
fn a_global_is_located_at_its_namespace_and_its_members_resolve() {
    let (fixture, loaded) = load(&Bundle::acme());

    let profile = &loaded.profiles[0];
    let run = profile.namespace("run").expect("`run` is declared");
    assert_eq!(run.location, fixture.path("host/schemas/run.yaml"));
    assert_eq!(run.at, Some((2, 12)), "at the `namespace` value");
    let global = profile.globals().global("run").expect("the `run` global");
    assert!(global.member("count").is_some(), "{:?}", global.definition());
    assert!(global.member("missing").is_none());
    assert_eq!(global.description(), Some("The current pass."));
    assert!(profile.namespace("loop").is_none());
}

/// A namespace importing a bundled library: the library is in the digest
/// map, so editing it (a legitimate republish) changes `catalog_hash` and
/// leaves an artifact that was not regenerated stale.
#[test]
fn the_catalog_hash_covers_a_transitively_imported_library() {
    let run = "kind: namespace-schema\nnamespace: run\n$schema:\n    count: number\n    widget: widget@./library.yaml\n";
    let covered = catalog_hash_of(&[("schemas/library.yaml", LIBRARY), ("schemas/run.yaml", run)]);
    assert_ne!(covered, catalog_hash_of(&[("schemas/run.yaml", run)]));
    let bundle = Bundle::acme()
        .with_member("schemas/run.yaml", run)
        .with_member(LIFECYCLE, &lifecycle_artifact_declaring("acme", &["run"], &covered));
    let (fixture, loaded) = load(&bundle);
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    assert_eq!(loaded.profiles[0].catalog_hash(), covered);
    assert!(loaded.profiles[0].globals().global("run").unwrap().member("widget").is_some(), "the import resolved");

    let edited = LIBRARY.replace("The widget's size.", "Its size.");
    fixture.write_member("host", "schemas/library.yaml", &edited);
    let diagnostic = refused(&fixture.load()).clone();
    assert_eq!(diagnostic.file, fixture.path(&format!("host/{LIFECYCLE}")));
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"));
    assert!(diagnostic.message.contains("regenerate the artifact"), "{}", diagnostic.message);

    let republished = catalog_hash_of(&[("schemas/library.yaml", &edited), ("schemas/run.yaml", run)]);
    fixture.write_member("host", LIFECYCLE, &lifecycle_artifact_declaring("acme", &["run"], &republished));
    let reloaded = fixture.load();
    assert!(reloaded.diagnostics.is_empty(), "{:#?}", reloaded.diagnostics);
    assert_eq!(reloaded.profiles[0].catalog_hash(), republished);
    assert_ne!(republished, covered);
}

#[test]
fn one_bundle_installed_in_two_folders_has_one_identity() {
    let fixture = Fixture::new();
    fixture.install("elsewhere/copy");
    let loaded = fixture.load_references(&["../host/darkmatter-host.yaml", "../elsewhere/copy/darkmatter-host.yaml"]);
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [first, second] = loaded.profiles.as_slice() else { panic!("two profiles: {:?}", loaded.profiles) };
    assert_ne!(first.manifest().path, second.manifest().path);
    assert_eq!(first.revision(), second.revision());
    assert_eq!(first.catalog_hash(), second.catalog_hash());
}

// ── the namespace envelope ──────────────────────────────────────────────────

#[test]
fn an_empty_payload_empty_types_and_absent_optional_fields_are_valid() {
    for text in [
        "kind: namespace-schema\nnamespace: run\ntypes: {}\n$schema: {}\n",
        "kind: namespace-schema\nnamespace: run\n$schema: {}\n",
        "kind: namespace-schema\nnamespace: run\n$schema:\n    count: number\n",
        "kind: namespace-schema\nnamespace: run\ntypes:\n    pass: number\n$schema:\n    count: pass\n",
    ] {
        let (_fixture, loaded) = load(&with_run(text));
        assert!(loaded.diagnostics.is_empty(), "{text}: {:#?}", loaded.diagnostics);
        assert!(loaded.profiles[0].globals().global("run").is_some(), "{text}");
    }
}

#[test]
fn an_explicit_null_or_a_wrong_shape_in_the_envelope_is_refused_in_its_file() {
    for text in [
        "kind: namespace-schema\nnamespace: run\ndescription: null\n$schema:\n    count: number\n",
        "kind: namespace-schema\nnamespace: run\ntypes: null\n$schema:\n    count: number\n",
        "kind: namespace-schema\nnamespace: run\n$schema: null\n",
        "kind: namespace-schema\nnamespace: null\n$schema:\n    count: number\n",
        "kind: namespace-schema\nnamespace: run\n$schema: [count]\n",
        "kind: namespace-schema\nnamespace: run\ntypes: [pass]\n$schema:\n    count: number\n",
        "kind: namespace-schema\n$schema:\n    count: number\n",
        "kind: namespace-schema\nnamespace: run\n$schema:\n    count: number\nextra: 1\n",
    ] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/run.yaml", text);
        refused_in_run(&fixture, &fixture.load());
    }
}

#[test]
fn a_reserved_or_malformed_namespace_name_is_refused_at_the_name() {
    for name in ["fm", "ctx", "env", "current", "local", "err", "timing", "\"null\"", "9run", "run.count"] {
        let fixture = Fixture::new();
        fixture.write_member("host", "schemas/run.yaml", &RUN_NAMESPACE.replace("namespace: run", &format!("namespace: {name}")));
        let diagnostic = refused_in_run(&fixture, &fixture.load()).clone();
        assert_eq!(diagnostic.line, Some(2), "{name}: {diagnostic:#?}");
    }
}

#[test]
fn a_namespace_declared_twice_is_refused_on_every_declaration() {
    let fixture = Fixture::new();
    fixture.install_bundle("host", &Bundle::acme().with_file("schemas/again.yaml", RUN_NAMESPACE));

    let loaded = fixture.load();

    assert!(loaded.profiles.is_empty());
    let mut located: Vec<(std::path::PathBuf, Option<usize>, Option<String>)> =
        loaded.diagnostics.iter().map(|diagnostic| (diagnostic.file.clone(), diagnostic.line, diagnostic.member.clone())).collect();
    located.sort();
    assert_eq!(
        located,
        [
            (fixture.path("host/schemas/again.yaml"), Some(2), Some("files[4]".to_string())),
            (fixture.path("host/schemas/run.yaml"), Some(2), Some("files[3]".to_string())),
        ]
    );
    assert!(loaded.diagnostics.iter().all(|diagnostic| diagnostic.message.contains("namespace `run` is also declared by")));
}

// ── the artifact's binding ──────────────────────────────────────────────────

#[test]
fn an_artifact_binding_a_namespace_the_catalog_lacks_is_refused() {
    let hash = catalog_hash_of(&[("schemas/run.yaml", RUN_NAMESPACE)]);
    let (fixture, loaded) = load(&Bundle::acme().with_member(LIFECYCLE, &lifecycle_artifact_declaring("acme", &["run", "walk"], &hash)));
    let diagnostic = refused(&loaded);
    assert_eq!(diagnostic.file, fixture.path(&format!("host/{LIFECYCLE}")));
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"));
    assert!(diagnostic.message.contains("binds host global `walk`"), "{}", diagnostic.message);
}

#[test]
fn an_artifact_of_another_host_is_refused_at_the_manifest_host() {
    let (fixture, loaded) =
        load(&Bundle::acme().with_member(LIFECYCLE, &lifecycle_artifact_for("other", &[("schemas/run.yaml", RUN_NAMESPACE)])));
    let diagnostic = refused(&loaded);
    assert_eq!(diagnostic.file, fixture.manifest());
    assert_eq!(diagnostic.member.as_deref(), Some("host"));
    assert!(diagnostic.message.contains("records host `other`"), "{}", diagnostic.message);
}

// ── binding roots are namespace members ─────────────────────────────────────

/// The relay bundle (shaped like Claudine's) with its lifecycle artifact
/// edited by `edit` and the manifest's digest refreshed; the namespace files,
/// and so the artifact's `catalog_hash`, are unchanged.
fn relay_with_artifact(edit: impl FnOnce(&mut serde_json::Value)) -> Bundle {
    let mut artifact: serde_json::Value =
        serde_json::from_str(&relay_lifecycle_artifact()).expect("the relay artifact is JSON");
    edit(&mut artifact);
    let mut text = serde_json::to_string_pretty(&artifact).expect("JSON serializes");
    text.push('\n');
    Bundle::relay().with_member(LIFECYCLE, &text)
}

/// The `events` entry the host spells `name`.
fn event<'a>(artifact: &'a mut serde_json::Value, name: &str) -> &'a mut serde_json::Value {
    artifact["events"]
        .as_array_mut()
        .expect("`events` is an array")
        .iter_mut()
        .find(|entry| entry["name"] == name)
        .expect("the event is listed")
}

/// Renames the declaration `from` of event `name` to `to`.
fn rename_declaration(artifact: &mut serde_json::Value, name: &str, from: &str, to: &str) {
    let declarations = event(artifact, name)["globals"]["declarations"].as_array_mut().expect("declarations");
    let declaration = declarations.iter_mut().find(|declaration| declaration["root"] == from).expect("declared");
    declaration["root"] = serde_json::Value::from(to);
}

/// The one refusal, in the lifecycle artifact at the line and column of the
/// only `"typo_root"` string in it.
fn refused_at_typo_root(bundle: &Bundle) {
    let (fixture, loaded) = load(bundle);
    let diagnostic = refused(&loaded);
    assert_eq!(diagnostic.file, fixture.path(&format!("host/{LIFECYCLE}")), "{diagnostic:#?}");
    assert_eq!(diagnostic.member.as_deref(), Some("lifecycle"), "{diagnostic:#?}");
    assert!(diagnostic.message.contains("binds `typo_root`"), "{}", diagnostic.message);
    let text = &bundle.members[LIFECYCLE];
    let (line, column) = text
        .lines()
        .enumerate()
        .find_map(|(index, line)| line.find("\"typo_root\"").map(|at| (index + 1, line[..at].chars().count() + 1)))
        .expect("the edit is in the artifact");
    assert_eq!((diagnostic.line, diagnostic.column), (Some(line), Some(column)), "{diagnostic:#?}");
}

#[test]
fn an_undeclared_root_on_a_darkmatter_event_is_refused_at_the_binding() {
    refused_at_typo_root(&relay_with_artifact(|artifact| rename_declaration(artifact, "initialize", "timing", "typo_root")));
}

#[test]
fn an_undeclared_root_on_a_host_event_is_refused_at_the_binding() {
    refused_at_typo_root(&relay_with_artifact(|artifact| rename_declaration(artifact, "success", "timing", "typo_root")));
}

#[test]
fn an_undeclared_host_global_is_refused_even_without_an_attached_catalog() {
    let detached = |artifact: &mut serde_json::Value| artifact["global_catalog"] = serde_json::Value::Bool(false);
    let (_fixture, control) = load(&relay_with_artifact(detached));
    assert!(control.diagnostics.is_empty(), "{:#?}", control.diagnostics);
    assert_eq!(control.profiles.len(), 1);

    refused_at_typo_root(&relay_with_artifact(|artifact| {
        detached(artifact);
        artifact["host_globals"] = serde_json::json!(["typo_root"]);
    }));
}

#[test]
fn darkmatter_err_and_timing_bindings_load_unedited() {
    let (_fixture, loaded) = load(&relay_with_artifact(|_| {}));
    assert!(loaded.diagnostics.is_empty(), "{:#?}", loaded.diagnostics);
    let [profile] = loaded.profiles.as_slice() else { panic!("one profile") };
    let registry = profile.lifecycle().registry();
    let id = registry.id("success").expect("the relay registers `success`");
    let roots: Vec<&str> = registry.globals(id).globals().map(|global| global.root()).collect();
    assert_eq!(roots, ["err", "timing"]);
}
