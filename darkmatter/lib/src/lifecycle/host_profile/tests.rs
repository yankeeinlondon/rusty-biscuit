use super::*;
use crate::lifecycle::registry::generic_view;
use crate::markdown::schemas::catalog::{GlobalCatalog, GlobalRequirement};

const CATALOG_PATH: &str = "host/globals.yaml";
const CATALOG_TEXT: &str = "kind: schema\ndescription: host\n$schema:\n    run:\n        count: \"number -> The pass.\"\n";

fn catalog() -> &'static GlobalCatalog {
    static CATALOG: std::sync::OnceLock<GlobalCatalog> = std::sync::OnceLock::new();
    CATALOG.get_or_init(|| {
        GlobalCatalog::from_documents(CATALOG_PATH, &[(CATALOG_PATH, CATALOG_TEXT)], &[GlobalRequirement::shape("run")])
            .expect("the test catalog loads")
    })
}

/// A host that renames `composed`, registers an outcome pair and an
/// iteration gate, declares a late global, and types it with its catalog.
fn host_registry() -> EventRegistry {
    let mut registry = EventRegistry::new("test");
    registry.rename(CoreEvent::Composed, "begin").unwrap();
    registry
        .declare_core_globals(CoreEvent::Composed, generic_view("test.begin", Availability::Unavailable(UnavailabilityReason::new("test.no-error").with_parameter("why", json!(1)))))
        .unwrap();
    for (name, position, role) in [
        ("won", Position::AfterComposition, Role::TerminalOutcome),
        ("lost", Position::AfterComposition, Role::TerminalOutcome),
        ("again", Position::AfterFinalize, Role::IterationGate),
    ] {
        registry
            .register(EventDescriptor::new(name, position, role, generic_view(&format!("test.{name}"), Availability::Available)))
            .unwrap();
    }
    registry.register_outcome_pair(OutcomePair::new("won", "lost").downgrade("won", "lost")).unwrap();
    registry.declare_host_global("run").unwrap();
    registry.register_global_catalog(catalog()).unwrap();
    registry.validate().unwrap();
    registry
}

fn options(registry: &EventRegistry) -> LifecycleParseOptions<'_> {
    LifecycleParseOptions {
        is_known_effect: &|name| name == "chime",
        authored: None,
        unknown_verbs: UnknownVerbPolicy::IgnoreUnprovided((&["say"]).into()),
        registry,
    }
}

fn artifact(registry: &EventRegistry) -> String {
    lifecycle_artifact(&options(registry), &["chime", "unknown-one"], CATALOG_TEXT)
}

fn load(text: &str) -> Result<HostLifecycleProfile, String> {
    HostLifecycleProfile::from_artifact(catalog(), Path::new(CATALOG_PATH), CATALOG_TEXT, FactsArtifact::Text(text))
}

#[test]
fn a_profile_round_trips_to_an_equal_registry_and_the_same_parse_options() {
    let registry = host_registry();
    let text = artifact(&registry);
    assert!(text.ends_with("}\n"));
    let profile = load(&text).expect("the artifact reads");
    assert_eq!(profile.registry().as_ref(), &registry);
    let rebuilt = profile.parse_options(None);
    assert_eq!(rebuilt.unknown_verbs, UnknownVerbPolicy::IgnoreUnprovided((&["say"]).into()));
    assert!((rebuilt.is_known_effect)("chime") && (rebuilt.is_known_effect)("Chime"));
    assert!(!(rebuilt.is_known_effect)("unknown-one"), "only accepted names are recorded");
    // Writing the rebuilt profile gives the same bytes.
    assert_eq!(lifecycle_artifact(&rebuilt, &["chime", "unknown-one"], CATALOG_TEXT), text);
}

#[test]
fn the_md_profile_round_trips() {
    let md = crate::lifecycle::md_host::parse_options(None);
    let text = lifecycle_artifact(&md, &[], CATALOG_TEXT);
    let profile = load(&text).expect("the md artifact reads");
    assert_eq!(profile.registry().as_ref(), md.registry);
    assert_eq!(profile.parse_options(None).unknown_verbs, md.unknown_verbs);
}

#[test]
fn the_artifact_sits_beside_its_catalog() {
    assert_eq!(lifecycle_artifact_path(Path::new("a/globals.yaml")), PathBuf::from("a/globals.lifecycle.json"));
}

/// Each problem is a reason naming the artifact; nothing is defaulted.
#[test]
fn an_absent_stale_or_lacking_artifact_is_refused_with_its_reason() {
    let text = artifact(&host_registry());
    let edit = |from: &str, to: &str| {
        assert!(text.contains(from), "{from}");
        text.replacen(from, to, 1)
    };
    let absent = HostLifecycleProfile::from_artifact(catalog(), Path::new(CATALOG_PATH), CATALOG_TEXT, FactsArtifact::Absent);
    let cases: Vec<(String, Result<HostLifecycleProfile, String>)> = vec![
        ("is absent".into(), absent),
        (
            "regenerate the artifact".into(),
            HostLifecycleProfile::from_artifact(catalog(), Path::new(CATALOG_PATH), "kind: schema\n", FactsArtifact::Text(&text)),
        ),
        ("unknown `format_version` 2".into(), load(&edit("\"format_version\": 1", "\"format_version\": 2"))),
        // An outcome alternative whose event the artifact lacks.
        ("refuses".into(), load(&edit("\"name\": \"lost\"", "\"name\": \"dropped\""))),
        ("unknown field `extra`".into(), load(&edit("\"host\": \"test\"", "\"host\": \"test\", \"extra\": 1"))),
        ("duplicate key".into(), load(&edit("\"host\": \"test\"", "\"host\": \"test\", \"host\": \"x\""))),
        ("`global_catalog` a string".into(), load(&edit("\"global_catalog\": true", "\"global_catalog\": \"yes\""))),
        ("has no `effects` field".into(), load(&edit("\"effects\": [\n    \"chime\"\n  ],", ""))),
    ];
    for (needle, result) in cases {
        let reason = result.expect_err(&needle);
        assert!(reason.starts_with("the lifecycle artifact `host/globals.lifecycle.json`"), "{reason}");
        assert!(reason.contains(&needle), "{needle}: {reason}");
    }
}

/// Owned host-global names, an owned catalog, and an owned
/// `ignore_unprovided` list write exactly the bytes their static spellings
/// write, and the profile read back from them (which owns its data) writes
/// those bytes again.
#[test]
fn owned_globals_and_ignored_verbs_round_trip_with_unchanged_bytes() {
    let statics = artifact(&host_registry());
    let mut owned = EventRegistry::new("test");
    owned.rename(CoreEvent::Composed, "begin").unwrap();
    owned
        .declare_core_globals(CoreEvent::Composed, generic_view("test.begin", Availability::Unavailable(UnavailabilityReason::new("test.no-error").with_parameter("why", json!(1)))))
        .unwrap();
    for (name, position, role) in [
        ("won", Position::AfterComposition, Role::TerminalOutcome),
        ("lost", Position::AfterComposition, Role::TerminalOutcome),
        ("again", Position::AfterFinalize, Role::IterationGate),
    ] {
        owned
            .register(EventDescriptor::new(name, position, role, generic_view(&format!("test.{name}"), Availability::Available)))
            .unwrap();
    }
    owned.register_outcome_pair(OutcomePair::new("won", "lost").downgrade("won", "lost")).unwrap();
    owned.declare_host_global(String::from("run")).unwrap();
    owned.register_global_catalog(Arc::new(catalog().clone())).unwrap();
    owned.validate().unwrap();
    assert_eq!(owned, host_registry(), "owned and static registrations are equal");
    let options = LifecycleParseOptions {
        is_known_effect: &|name| name == "chime",
        authored: None,
        unknown_verbs: UnknownVerbPolicy::IgnoreUnprovided(vec![String::from("say")].into()),
        registry: &owned,
    };
    let text = lifecycle_artifact(&options, &["chime", "unknown-one"], CATALOG_TEXT);
    assert_eq!(text, statics);

    let profile = load(&text).expect("the artifact reads");
    let rebuilt = profile.parse_options(None);
    assert!(matches!(&rebuilt.unknown_verbs, UnknownVerbPolicy::IgnoreUnprovided(crate::lifecycle::VerbList::Shared(_))));
    assert!(rebuilt.unknown_verbs.ignores("say") && !rebuilt.unknown_verbs.ignores("shell"));
    assert_eq!(profile.registry().host_globals(), ["err", "timing", "run"]);
    assert_eq!(lifecycle_artifact(&rebuilt, &["chime", "unknown-one"], CATALOG_TEXT), statics);
}

/// The two owned collections, `host_globals` and
/// `unknown_verbs.ignore_unprovided`: absent, null, a wrong whole-field type,
/// a mixed list, and a wholly invalid list are refused naming the field or
/// element; an empty list is valid.
#[test]
fn owned_collection_fields_refuse_every_invalid_shape_and_admit_an_empty_list() {
    let text = artifact(&host_registry());
    let with = |edit: &dyn Fn(&mut serde_json::Map<String, Value>)| {
        let mut root: Value = serde_json::from_str(&text).unwrap();
        edit(root.as_object_mut().unwrap());
        load(&root.to_string())
    };
    let verbs = |list: Value| move |root: &mut serde_json::Map<String, Value>| {
        root.insert("unknown_verbs".to_string(), json!({ "ignore_unprovided": list.clone() }));
    };
    let globals = |list: Value| move |root: &mut serde_json::Map<String, Value>| {
        root.insert("host_globals".to_string(), list.clone());
    };
    let refused: Vec<(&str, Result<HostLifecycleProfile, String>)> = vec![
        ("has no `host_globals` field", with(&|root| drop(root.remove("host_globals")))),
        ("has `host_globals: null`", with(&globals(Value::Null))),
        ("`host_globals` a string instead of an array", with(&globals(json!("run")))),
        ("`host_globals[1]` a number instead of a string", with(&globals(json!(["run", 7])))),
        ("`host_globals[0]` a number instead of a string", with(&globals(json!([7])))),
        ("has no `unknown_verbs.ignore_unprovided` field", with(&|root| {
            root.insert("unknown_verbs".to_string(), json!({}));
        })),
        ("has `unknown_verbs.ignore_unprovided: null`", with(&verbs(Value::Null))),
        ("`unknown_verbs.ignore_unprovided` a string instead of an array", with(&verbs(json!("say")))),
        ("`unknown_verbs.ignore_unprovided[1]` a number", with(&verbs(json!(["say", 7])))),
        ("`unknown_verbs.ignore_unprovided[0]` a number", with(&verbs(json!([7])))),
    ];
    for (needle, result) in refused {
        let reason = result.expect_err(needle);
        assert!(reason.contains(needle), "{needle}: {reason}");
    }

    let empty_verbs = with(&verbs(json!([]))).expect("an empty verb list is valid");
    assert!(!empty_verbs.parse_options(None).unknown_verbs.ignores("say"));
    let empty_globals = with(&globals(json!([]))).expect("an empty host-global list is valid");
    assert_eq!(empty_globals.registry().host_globals(), ["err", "timing"]);
}
