//! Claudine's host catalog: the `seq`, `loop`, and `group` shapes, one
//! `kind: namespace-schema` member of `claudine/registration/` each.
//!
//! The catalog is prepared from those members by Darkmatter's namespace
//! preparation, its members are exactly the keys of the runtime values
//! Claudine binds, every lifecycle view declares exactly its globals, and a
//! namespace file is never applied as a trigger. The namespace files' own
//! input robustness is Darkmatter's (`prepare_catalog`); the lifecycle
//! artifact versioned against the catalog is checked at the end.

use simplified_schema::triggers::TriggerOrigin;
use std::collections::{BTreeSet, HashMap};

use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle::bindings::{
    GroupBinding, GroupLegResult, GroupMembership, LegOutcome, LifecycleScope, LoopMembership, SequenceBinding,
    binding_view_in,
};
use claudine::composition::lifecycle::registry::claudine_registry;
use claudine::composition::looping::LoopAmbient;
use claudine::composition::schema::bundle::with_claudine_registration;
use claudine::composition::inventory::{self, MemberKind};
use claudine::composition::schema::globals::{CLAUDINE_GLOBALS, claudine_globals, prepare_namespace_catalog};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::catalog::GlobalCatalog;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use simplified_schema::PatternKey;
use serde_json::{Value, json};

/// The literal member names `global` declares.
fn members(catalog: &GlobalCatalog, global: &str) -> BTreeSet<String> {
    catalog
        .global(global)
        .and_then(|declaration| declaration.shape())
        .unwrap_or_else(|| panic!("`{global}` is a shape"))
        .properties
        .keys()
        .cloned()
        .collect()
}

fn keys(value: &Value) -> BTreeSet<String> {
    value.as_object().expect("an object").keys().cloned().collect()
}

#[test]
fn the_catalog_is_prepared_from_the_namespace_members_and_declares_group_loop_and_seq() {
    let namespaces: Vec<&str> = inventory::members_of(MemberKind::Namespace).map(|member| member.path).collect();
    assert_eq!(namespaces, ["group.yaml", "loop.yaml", "seq.yaml"]);
    let names: Vec<_> = claudine_globals().globals().map(|global| global.name().to_string()).collect();
    let required: Vec<_> = CLAUDINE_GLOBALS.iter().map(|global| global.name.to_string()).collect();
    assert_eq!(names, ["group", "loop", "seq"]);
    assert_eq!(names, required);
    let prepared = prepare_namespace_catalog().expect("the namespace members prepare");
    assert_eq!(&*prepared.catalog, claudine_globals(), "the shipped entry is the prepared catalog");
    let digested: Vec<&str> = prepared.digests.keys().map(String::as_str).collect();
    assert_eq!(digested, ["schemas/group.yaml", "schemas/loop.yaml", "schemas/seq.yaml"], "they import nothing");
}

/// Each declared shape has exactly the members of the value Claudine binds;
/// authored step fields and group variables are the declared open keys.
#[test]
fn every_shape_matches_the_runtime_value() {
    let catalog = claudine_globals();
    assert_eq!(members(catalog, "seq"), keys(&SequenceBinding::container("run", Vec::new()).to_value()));
    assert_eq!(members(catalog, "loop"), keys(&LoopAmbient::new(1, true, false, "", 0).to_value()));
    let group = GroupBinding {
        variables: serde_json::Map::new(),
        legs: vec![GroupLegResult {
            name: "lint".into(),
            outcome: LegOutcome::Failed,
            output: json!("partial"),
            err: json!({"code": "x"}),
        }],
    };
    let settled = Value::Object(group.event_value(LifecycleSignal::Finalize));
    assert_eq!(members(catalog, "group"), keys(&settled));
    let legs = catalog.global("group").and_then(|group| group.member("legs")).expect("legs");
    let simplified_schema::PropertyDef::Single(legs) = legs else { panic!("legs is one atom") };
    let simplified_schema::TypeExpr::InlineObject(legs) = &legs.ty else { panic!("legs is a shape") };
    let leg = &legs.pattern_keys.first().expect("legs are keyed by name");
    assert_eq!(leg.key, PatternKey::CatchAll);
    let simplified_schema::PropertyDef::Single(leg) = &leg.def else { panic!("a leg is one atom") };
    let simplified_schema::TypeExpr::InlineObject(leg) = &leg.ty else { panic!("a leg is a shape") };
    let declared: BTreeSet<_> = leg.properties.keys().cloned().collect();
    assert_eq!(declared, keys(&settled["legs"]["lint"]));
    for global in ["seq", "group"] {
        let shape = catalog.global(global).and_then(|declaration| declaration.shape()).expect("shape");
        assert!(shape.pattern_keys.iter().any(|pattern| pattern.key == PatternKey::CatchAll), "{global} is open");
    }
}

/// Every lifecycle view declares exactly the catalog's globals beside
/// Darkmatter's `err` and `timing`, and the registry's late globals are
/// catalog globals.
#[test]
fn every_binding_view_declares_exactly_the_catalog_globals() {
    let catalog: BTreeSet<String> = claudine_globals().globals().map(|global| global.name().to_string()).collect();
    for scope in LifecycleScope::ALL {
        for group in [GroupMembership::Member, GroupMembership::Outside, GroupMembership::Unknown] {
            for in_loop in [LoopMembership::Member, LoopMembership::Outside, LoopMembership::Unknown] {
                let view = binding_view_in(scope, group, in_loop);
                let declared: BTreeSet<String> = view
                    .globals()
                    .map(|global| global.root().to_string())
                    .filter(|root| root != "err" && root != "timing")
                    .collect();
                assert_eq!(declared, catalog, "{scope:?}");
            }
        }
    }
    for global in claudine_registry().host_globals() {
        assert!(catalog.contains(global.as_ref()) || ["err", "timing"].contains(&global.as_ref()), "{global}");
    }
}

/// With the namespace files in a repository's `schemas/` root and
/// Claudine's compiled registration as the registered host, a namespace file
/// is no trigger in either place: a document gains no property from it. Only
/// host registration makes its global a global.
#[test]
fn a_namespace_file_is_never_a_trigger() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    let namespaces: Vec<(&str, &str)> =
        inventory::members_of(MemberKind::Namespace).map(|member| (member.path, member.text)).collect();
    for (name, text) in &namespaces {
        std::fs::write(root.join("schemas").join(name), text).expect("schema file");
    }
    let context =
        biscuit_file::FileResolutionContext::from_snapshot(&root, None, HashMap::new()).with_repository_root(&root);
    let schemas =
        with_claudine_registration(DarkmatterSchemas::new(context)).with_trigger_discovery().expect("the root's triggers load");
    let effective = |frontmatter: &str| {
        let path = root.join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("doc");
        let markdown = Markdown::try_from(path.as_path()).expect("loads").with_source(ComposeSource::File(path));
        schemas.effective_for(&markdown)
    };
    let selected = effective("prompt: hi\n").expect("prepares").expect("Claudine's trigger applies");
    for origin in selected.origins.values() {
        let named = origin.trigger_source().and_then(|source| source.file_name().map(str::to_string));
        let named = named.as_deref().map(|name| name.rsplit('/').next().unwrap_or(name));
        assert!(!namespaces.iter().any(|(name, _)| named == Some(*name)), "{origin:?}");
    }
    assert!(!selected.declares_top_level_property("seq") && !selected.declares_top_level_property("group"));
}

/// Claudine extends the expression language with no host function: no file in
/// `claudine/schemas/` is a function catalog, and no `.facts.json` artifact
/// sits there. A host function needs both, registered together: its
/// `function(...)` declaration in a catalog there, its Rust registration
/// through `darkmatter::markdown::compose::expression::host::HostFunctions`,
/// and the artifact that registration writes
/// (`HostFunctions::facts_artifact`), kept fresh by a test comparing the two
/// byte for byte. This test fails when a catalog or an artifact appears
/// without that wiring. The host lifecycle artifact
/// (`claudine/lib/generated/globals.lifecycle.json`) is a different artifact,
/// about events rather than functions; [`lifecycle_artifact`] keeps it fresh.
#[test]
fn claudine_ships_no_host_function_catalog_and_no_facts_artifact() {
    let schemas = biscuit_test_harness::manifest_dir!().join("../schemas");
    let mut catalogs = Vec::new();
    let mut artifacts = Vec::new();
    for entry in std::fs::read_dir(&schemas).expect("claudine/schemas is readable") {
        let path = entry.expect("a directory entry").path();
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string();
        if name.ends_with(".facts.json") {
            artifacts.push(name);
        } else if name.ends_with(".yaml") || name.ends_with(".yml") {
            let text = std::fs::read_to_string(&path).expect("a schema file is text");
            if darkmatter::markdown::schemas::catalog::is_host_function_catalog(&text) {
                catalogs.push(name);
            }
        }
    }
    assert!(
        catalogs.is_empty() && artifacts.is_empty(),
        "claudine/schemas holds host function catalogs {catalogs:?} or facts artifacts {artifacts:?}; register \
         each function through `HostFunctions`, install it at startup, and add a freshness test for its artifact"
    );
}

// ── The host lifecycle artifact ─────────────────────────────────────────────

mod lifecycle_artifact {
    use std::path::Path;

    use claudine::composition::lifecycle::registry::claudine_registry;
    use claudine::composition::lifecycle::{
        LIFECYCLE_ARTIFACT_PATH, check_lifecycle_artifact, lifecycle_artifact, reject_incompatible_calls,
    };
    use claudine::composition::parse_lifecycle_config;
    use claudine::composition::schema::globals::prepare_namespace_catalog;
    use darkmatter::lifecycle::UnknownVerbPolicy;
    use darkmatter::lifecycle::host_profile::HostLifecycleProfile;
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::expression::host::FactsArtifact;
    use darkmatter::markdown::schemas::DarkmatterSchemas;

    /// The committed artifact.
    const COMMITTED: &str = include_str!("../../../lib/generated/globals.lifecycle.json");

    fn profile() -> HostLifecycleProfile {
        let catalog = prepare_namespace_catalog().expect("the namespace members prepare");
        HostLifecycleProfile::from_artifact_at(
            std::sync::Arc::clone(&catalog.catalog),
            &catalog.hash,
            Path::new(LIFECYCLE_ARTIFACT_PATH),
            FactsArtifact::Text(COMMITTED),
        )
        .expect("the committed artifact reads")
    }

    /// The freshness check: Claudine's registration and parse options, written
    /// now, are the committed bytes, which the library embeds for
    /// publication.
    #[test]
    fn the_committed_lifecycle_artifact_is_fresh() {
        // Spelled for CI's test-input index: the `catalog_hash` is computed
        // from these members by this code.
        let _ = include_str!("../../../registration/group.yaml");
        let _ = include_str!("../../../registration/loop.yaml");
        let _ = include_str!("../../../registration/seq.yaml");
        let _ = include_str!("../../../../darkmatter/lib/src/config/registration/namespaces.rs");
        assert!(
            lifecycle_artifact().expect("generates") == COMMITTED,
            "{LIFECYCLE_ARTIFACT_PATH} is stale; run `just shipped-schemas` from the repository root"
        );
        assert_eq!(claudine::composition::inventory::LIFECYCLE_ARTIFACT, COMMITTED.as_bytes());
    }

    /// The `catalog_hash` is Darkmatter's hash of the real namespace members,
    /// and the generator's check refuses an artifact derived from other
    /// namespace bytes (a stale derived member) or recording another host.
    #[test]
    fn the_lifecycle_check_refuses_an_artifact_versioned_against_other_namespaces() {
        use darkmatter::config::registration::namespaces::{member_digest, namespace_catalog_hash};
        let digests: std::collections::BTreeMap<String, String> = claudine::composition::inventory::members_of(
            claudine::composition::inventory::MemberKind::Namespace,
        )
        .map(|member| (member.bundle_path(), member_digest(member.bytes())))
        .collect();
        let hash = namespace_catalog_hash(&digests);
        assert!(COMMITTED.contains(&format!("\"catalog_hash\": \"{hash}\"")), "the hash covers the three namespace files");
        check_lifecycle_artifact(COMMITTED).expect("the committed artifact is current");

        let mut stale = digests.clone();
        stale.insert("schemas/seq.yaml".to_string(), member_digest(b"kind: namespace-schema\nnamespace: seq\n"));
        let stale_artifact = COMMITTED.replace(&hash, &namespace_catalog_hash(&stale));
        let refusal = check_lifecycle_artifact(&stale_artifact).expect_err("a stale catalog hash is refused");
        assert!(refusal.contains("catalog"), "{refusal}");

        let foreign = COMMITTED.replacen("\"host\": \"claudine\"", "\"host\": \"relay\"", 1);
        assert_ne!(foreign, COMMITTED, "the artifact records its host");
        assert!(check_lifecycle_artifact(&foreign).is_err());
    }

    /// One edit to the published artifact, each a root no namespace of
    /// Claudine's catalog declares, is refused as registration refuses it:
    /// on a Darkmatter event, on a Claudine event, and as a host global
    /// with no catalog attached. The unedited artifact's `err`/`timing`
    /// bindings are the control above.
    #[test]
    fn the_lifecycle_check_refuses_a_binding_no_namespace_declares() {
        fn edited(edit: impl FnOnce(&mut serde_json::Value)) -> String {
            let mut artifact: serde_json::Value = serde_json::from_str(COMMITTED).expect("JSON");
            edit(&mut artifact);
            let mut text = serde_json::to_string_pretty(&artifact).expect("serializes");
            text.push('\n');
            text
        }
        fn rename_timing(artifact: &mut serde_json::Value, event: &str) {
            let entry = artifact["events"].as_array_mut().unwrap().iter_mut().find(|entry| entry["name"] == event).unwrap();
            let timing = entry["globals"]["declarations"].as_array_mut().unwrap().iter_mut().find(|entry| entry["root"] == "timing").unwrap();
            timing["root"] = serde_json::json!("typo_root");
        }
        check_lifecycle_artifact(&edited(|_| {})).expect("a reserialized artifact is still current");
        for (cell, text) in [
            ("initialize", edited(|artifact| rename_timing(artifact, "initialize"))),
            ("success", edited(|artifact| rename_timing(artifact, "success"))),
            (
                "host_globals",
                edited(|artifact| {
                    artifact["global_catalog"] = serde_json::json!(false);
                    artifact["host_globals"][0] = serde_json::json!("typo_root");
                }),
            ),
        ] {
            let refusal = check_lifecycle_artifact(&text).expect_err(cell);
            assert!(refusal.contains("binds `typo_root`"), "{cell}: {refusal}");
        }
    }

    /// The artifact rebuilds exactly the registry composition uses, rejects
    /// unknown verbs as composition does, and knows the same effect names.
    #[test]
    fn the_artifact_rebuilds_claudines_registry_and_parse_options() {
        let profile = profile();
        assert_eq!(profile.registry(), claudine_registry());
        assert_eq!(profile.parse_options(None).unknown_verbs, UnknownVerbPolicy::Reject);
        for name in playa::SoundEffect::all_names() {
            for spelling in [name.to_string(), name.to_uppercase(), name.replace('-', "_")] {
                assert_eq!(
                    profile.is_known_effect(&spelling),
                    playa::SoundEffect::from_name(&spelling).is_some(),
                    "{spelling}"
                );
            }
        }
        assert!(!profile.is_known_effect("no-such-effect"));
    }

    fn frontmatter(text: &str) -> serde_json::Value {
        let md = Markdown::from(text);
        serde_json::Value::Object(md.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
    }

    /// Runtime parity for the guard → call → set → call shape in each
    /// Claudine-only event and verb: the walk under the rebuilt profile finds
    /// exactly what it finds under Claudine's registry, the only finding is
    /// the post-write call's, and composition's warnings are the walk's
    /// default-severity findings.
    #[test]
    fn the_rebuilt_profile_types_claudine_events_as_composition_does() {
        let dir = tempfile::tempdir().unwrap();
        let profile = profile();
        for event in ["start", "success", "blocked", "failure"] {
            for verb in ["stdout", "say"] {
                let text = format!(
                    "---\n$schema:\n  count: number\n  label: string(required)\ncount: 1\nlabel: x\n{event}:\n  - when: count\n    then:\n      - {verb}: \"{{{{ min(count, 5) }}}}\"\n      - set:\n          count: 7\n      - {verb}: \"{{{{ min(count, 6) }}}}\"\n      - {verb}: \"{{{{ min(label, 7) }}}}\"\n---\nBody\n"
                );
                let value = frontmatter(&text);
                let schema = DarkmatterSchemas::new(biscuit_file::FileResolutionContext::new(dir.path()))
                    .effective_for(&Markdown::from(text.as_str()))
                    .expect("the schema resolves")
                    .expect("an effective schema");
                let runtime_config = parse_lifecycle_config(&value, Path::new("doc.md")).expect("composition parses it");
                let editor_config = darkmatter::lifecycle::parse_lifecycle_config_with(
                    &value,
                    Path::new("doc.md"),
                    &profile.parse_options(None),
                )
                .expect("the profile parses it");
                let runtime =
                    darkmatter::lifecycle::lifecycle_call_checks(&value, &runtime_config, claudine_registry(), Some(&schema));
                let editor =
                    darkmatter::lifecycle::lifecycle_call_checks(&value, &editor_config, profile.registry(), Some(&schema));
                assert_eq!(editor, runtime, "{event}/{verb}");

                let strict: Vec<String> = editor
                    .iter()
                    .filter(|surface| surface.checks.iter().any(|found| !found.check.diagnostics(true).is_empty()))
                    .map(|surface| surface.property.to_string())
                    .collect();
                assert_eq!(
                    strict,
                    [format!("{event}[0].then[2].{verb}.text"), format!("{event}[0].then[3].{verb}.text")],
                    "{event}/{verb}: only the post-write and the string calls warn under strict"
                );
                let defaults: usize =
                    editor.iter().flat_map(|surface| &surface.checks).map(|found| found.check.diagnostics(false).len()).sum();
                let warnings = reject_incompatible_calls(&value, &runtime_config, Path::new("doc.md"), None, Some(&schema))
                    .expect("nothing is rejected");
                assert_eq!(warnings.len(), defaults, "{event}/{verb}: {warnings:#?}");
                assert_eq!(defaults, 1, "{event}/{verb}: the string argument is the one default-severity finding");
            }
        }
    }
}
