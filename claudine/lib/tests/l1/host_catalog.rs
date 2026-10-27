//! Claudine's host catalog, `claudine/schemas/globals.yaml`: the one
//! declaration of the `seq`, `loop`, and `group` shapes.
//!
//! The catalog's members are exactly the keys of the runtime values Claudine
//! binds, every lifecycle view declares exactly its globals, and the file is
//! never applied as a document schema or a trigger. The input robustness
//! matrix at the end edits one field of the shipped file per test, from an
//! unedited positive control, through Darkmatter's public catalog reader.

use std::collections::{BTreeSet, HashMap};

use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle::bindings::{
    GroupBinding, GroupLegResult, GroupMembership, LegOutcome, LifecycleScope, LoopMembership, SequenceBinding,
    binding_view_in,
};
use claudine::composition::lifecycle::registry::claudine_registry;
use claudine::composition::looping::LoopAmbient;
use claudine::composition::schema::bundle::CLAUDINE_SCHEMA_FILES;
use claudine::composition::schema::globals::{
    CLAUDINE_GLOBALS, HOST_CATALOG_PATH, HOST_CATALOG_TEXT, claudine_globals,
};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::catalog::{CatalogError, CatalogErrorReason, GlobalCatalog};
use darkmatter::markdown::schemas::{DarkmatterSchemas, PatternKey};
use serde_json::{Value, json};

fn load(text: &str) -> Result<GlobalCatalog, CatalogError> {
    GlobalCatalog::from_documents(HOST_CATALOG_PATH, &[(HOST_CATALOG_PATH, text)], CLAUDINE_GLOBALS)
}

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
fn the_embedded_catalog_is_the_repository_file_and_declares_seq_loop_and_group() {
    assert_eq!(HOST_CATALOG_TEXT, include_str!("../../../schemas/globals.yaml"));
    let names: Vec<_> = claudine_globals().globals().map(|global| global.name().to_string()).collect();
    assert_eq!(names, ["seq", "loop", "group"]);
    assert_eq!(&load(include_str!("../../../schemas/globals.yaml")).expect("loads"), claudine_globals());
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
    let darkmatter::markdown::schemas::PropertyDef::Single(legs) = legs else { panic!("legs is one atom") };
    let darkmatter::markdown::schemas::TypeExpr::InlineObject(legs) = &legs.ty else { panic!("legs is a shape") };
    let leg = &legs.pattern_keys.first().expect("legs are keyed by name");
    assert_eq!(leg.key, PatternKey::CatchAll);
    let darkmatter::markdown::schemas::PropertyDef::Single(leg) = &leg.def else { panic!("a leg is one atom") };
    let darkmatter::markdown::schemas::TypeExpr::InlineObject(leg) = &leg.ty else { panic!("a leg is a shape") };
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

/// With the bundle and the host catalog in a repository's `schemas/` root,
/// the catalog is no trigger and no document schema: a document gains no
/// property from it, and naming it fails.
#[test]
fn the_host_catalog_is_never_a_trigger_or_a_document_schema() {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical");
    std::fs::create_dir_all(root.join(".git")).expect("repository marker");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas");
    for (name, text) in CLAUDINE_SCHEMA_FILES.iter().chain([&("globals.yaml", HOST_CATALOG_TEXT)]) {
        std::fs::write(root.join("schemas").join(name), text).expect("schema file");
    }
    let context =
        biscuit_file::FileResolutionContext::from_snapshot(&root, None, HashMap::new()).with_repository_root(&root);
    let schemas = DarkmatterSchemas::new(context).with_trigger_discovery().expect("the root's triggers load");
    let effective = |frontmatter: &str| {
        let path = root.join("doc.md");
        std::fs::write(&path, format!("---\n{frontmatter}---\nBody\n")).expect("doc");
        let markdown = Markdown::try_from(path.as_path()).expect("loads").with_source(ComposeSource::File(path));
        schemas.effective_for(&markdown)
    };
    let selected = effective("prompt: hi\n").expect("prepares").expect("Claudine's trigger applies");
    for origin in selected.origins.values() {
        let named = origin.trigger_source().and_then(|source| source.file_name().map(str::to_string));
        assert_ne!(named.as_deref(), Some("globals.yaml"), "{origin:?}");
    }
    assert!(!selected.declares_top_level_property("seq") && !selected.declares_top_level_property("group"));
    let Err(error) = effective("$schema: globals.yaml\n") else {
        panic!("the host catalog applied as a document schema");
    };
    assert!(format!("{error:?}").contains("global catalog"), "{error:?}");
}

// ── input robustness matrix ────────────────────────────────────────────────

const COLUMNS: [&str; 5] = ["kind", "$schema", "seq", "loop", "group"];
const ROWS: [&str; 9] =
    ["control", "absent", "null", "wrong_type", "one_wrong", "every_wrong", "empty", "duplicate", "trailing"];

#[derive(Clone, Copy, Debug)]
enum Expect {
    Loads,
    Missing(&'static str),
    Null(&'static str),
    WrongType(&'static str),
    Empty(&'static str),
    Yaml,
    Resolve(&'static str),
}

fn replace_once(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` occurs once");
    text.replacen(from, to, 1)
}

/// The text of `global`'s block: from its key line to the next global's.
fn block<'a>(text: &'a str, global: &str) -> &'a str {
    let start = text.find(&format!("\n    {global}:\n")).expect("global block") + 1;
    let rest = &text[start + 1..];
    let end = ["\n    seq:\n", "\n    loop:\n", "\n    group:\n"]
        .iter()
        .filter_map(|next| rest.find(next).map(|at| start + 1 + at + 1))
        .min()
        .unwrap_or(text.len());
    &text[start..end]
}

/// `global`'s block with each member line (8-space indent) passed through `member`.
fn edit_members(text: &str, global: &str, member: impl Fn(&str) -> String) -> String {
    let original = block(text, global);
    let edited: String = original
        .lines()
        .map(|line| {
            let is_member = line.starts_with("        ") && !line.starts_with("         ");
            format!("{}\n", if is_member { member(line) } else { line.to_string() })
        })
        .collect();
    replace_once(text, original, &edited)
}

fn every_member_wrong(line: &str) -> String {
    format!("{}: strng", line.split(':').next().expect("key"))
}

fn check_cell(column: &str, row: &str, edit: impl Fn(&str) -> String, expect: Expect) {
    assert!(COLUMNS.contains(&column) && ROWS.contains(&row), "{column}/{row} is a matrix cell");
    let shipped = include_str!("../../../schemas/globals.yaml");
    let edited = edit(shipped);
    if row != "control" {
        assert_ne!(edited, shipped, "{column}/{row} edits the file");
    }
    let outcome = load(&edited);
    let reason = |error: &CatalogError, field: &str| {
        assert_eq!(error.field, field, "{column}/{row}: {error}");
        format!("{:?}", error.reason)
    };
    match (expect, &outcome) {
        (Expect::Loads, Ok(catalog)) => {
            assert_eq!(catalog, claudine_globals());
            assert_eq!(members(catalog, "loop"), keys(&LoopAmbient::new(2, false, true, "", 0).to_value()));
        }
        (Expect::Missing(field), Err(error)) => {
            reason(error, field);
            assert!(matches!(error.reason, CatalogErrorReason::Missing), "{column}/{row}: {error}");
        }
        (Expect::Null(field), Err(error)) => {
            reason(error, field);
            assert!(matches!(error.reason, CatalogErrorReason::Null), "{column}/{row}: {error}");
        }
        (Expect::WrongType(field), Err(error)) => {
            reason(error, field);
            assert!(matches!(error.reason, CatalogErrorReason::WrongType { .. }), "{column}/{row}: {error}");
        }
        (Expect::Empty(field), Err(error)) => {
            reason(error, field);
            assert!(matches!(error.reason, CatalogErrorReason::Empty), "{column}/{row}: {error}");
        }
        (Expect::Yaml, Err(error)) => {
            assert!(matches!(error.reason, CatalogErrorReason::Yaml(_)), "{column}/{row}: {error}");
        }
        (Expect::Resolve(field), Err(error)) => {
            reason(error, field);
            assert!(matches!(error.reason, CatalogErrorReason::Resolve(_)), "{column}/{row}: {error}");
        }
        (expect, outcome) => panic!("{column}/{row}: expected {expect:?}, got {:?}", outcome.as_ref().map(|_| "loaded")),
    }
}

macro_rules! matrix {
    ($($test:ident: $column:literal / $row:literal, $edit:expr => $expect:expr;)+) => {
        $(
            #[test]
            fn $test() {
                check_cell($column, $row, $edit, $expect);
            }
        )+

        /// The generated tests are exactly the plan's host-catalog cells.
        #[test]
        fn the_matrix_tests_cover_every_host_catalog_cell_once() {
            let generated: Vec<(&str, &str)> = vec![$(($column, $row)),+];
            let unique: BTreeSet<_> = generated.iter().copied().collect();
            assert_eq!(unique.len(), generated.len(), "a cell is generated twice");
            let table: BTreeSet<_> =
                COLUMNS.iter().flat_map(|column| ROWS.iter().map(move |row| (*column, *row))).collect();
            assert_eq!(unique, table);
        }
    };
}

matrix! {
    // ── kind ──
    kind_control: "kind" / "control", |t| t.to_string() => Expect::Loads;
    kind_absent: "kind" / "absent", |t| replace_once(t, "kind: schema\n", "") => Expect::Missing("kind");
    kind_null: "kind" / "null", |t| replace_once(t, "kind: schema\n", "kind: null\n") => Expect::Null("kind");
    kind_wrong_type: "kind" / "wrong_type", |t| replace_once(t, "kind: schema\n", "kind: 3\n") => Expect::WrongType("kind");
    // N/A for a scalar: a collection substituted there fails.
    kind_one_wrong: "kind" / "one_wrong", |t| replace_once(t, "kind: schema\n", "kind: [schema, 3]\n") => Expect::WrongType("kind");
    kind_every_wrong: "kind" / "every_wrong", |t| replace_once(t, "kind: schema\n", "kind: [3, 4]\n") => Expect::WrongType("kind");
    kind_empty: "kind" / "empty", |t| replace_once(t, "kind: schema\n", "kind: \"\"\n") => Expect::Empty("kind");
    kind_duplicate: "kind" / "duplicate", |t| replace_once(t, "kind: schema\n", "kind: schema\nkind: schema\n") => Expect::Yaml;
    kind_trailing: "kind" / "trailing", |t| format!("{t}---\nkind: schema\n") => Expect::Yaml;
    // ── $schema ──
    schema_control: "$schema" / "control", |t| t.to_string() => Expect::Loads;
    schema_absent: "$schema" / "absent", |t| t[..t.find("$schema:\n").unwrap()].to_string() => Expect::Missing("$schema");
    schema_null: "$schema" / "null", |t| format!("{}$schema: null\n", &t[..t.find("$schema:\n").unwrap()]) => Expect::Null("$schema");
    schema_wrong_type: "$schema" / "wrong_type", |t| format!("{}$schema: [seq, loop]\n", &t[..t.find("$schema:\n").unwrap()]) => Expect::WrongType("$schema");
    schema_one_wrong: "$schema" / "one_wrong", |t| replace_once(t, block(t, "loop"), "    loop: \"{ count: nmber }\"\n") => Expect::Resolve("$schema.loop");
    schema_every_wrong: "$schema" / "every_wrong", |t| format!("{}$schema:\n    seq: 1\n    loop: 1\n    group: 1\n", &t[..t.find("$schema:\n").unwrap()]) => Expect::WrongType("$schema.seq");
    schema_empty: "$schema" / "empty", |t| format!("{}$schema: {{}}\n", &t[..t.find("$schema:\n").unwrap()]) => Expect::Empty("$schema");
    schema_duplicate: "$schema" / "duplicate", |t| format!("{t}$schema:\n    seq: object\n") => Expect::Yaml;
    schema_trailing: "$schema" / "trailing", |t| format!("{t}  : [\n") => Expect::Yaml;
    // ── seq ──
    seq_control: "seq" / "control", |t| t.to_string() => Expect::Loads;
    seq_absent: "seq" / "absent", |t| replace_once(t, block(t, "seq"), "") => Expect::Missing("$schema.seq");
    seq_null: "seq" / "null", |t| replace_once(t, block(t, "seq"), "    seq: null\n") => Expect::Null("$schema.seq");
    seq_wrong_type: "seq" / "wrong_type", |t| replace_once(t, block(t, "seq"), "    seq: 42\n") => Expect::WrongType("$schema.seq");
    seq_one_wrong: "seq" / "one_wrong", |t| replace_once(t, "index: \"number(integer; min(1))", "index: \"nmber(integer; min(1))") => Expect::Resolve("$schema.seq");
    seq_every_wrong: "seq" / "every_wrong", |t| edit_members(t, "seq", every_member_wrong) => Expect::Resolve("$schema.seq");
    seq_empty: "seq" / "empty", |t| replace_once(t, block(t, "seq"), "    seq: {}\n") => Expect::Empty("$schema.seq");
    seq_duplicate: "seq" / "duplicate", |t| edit_members(t, "seq", |line| if line.contains("run_id:") { format!("{line}\n{line}") } else { line.to_string() }) => Expect::Yaml;
    seq_trailing: "seq" / "trailing", |t| replace_once(t, block(t, "seq"), "    seq: [object\n") => Expect::Yaml;
    // ── loop ──
    loop_control: "loop" / "control", |t| t.to_string() => Expect::Loads;
    loop_absent: "loop" / "absent", |t| replace_once(t, block(t, "loop"), "") => Expect::Missing("$schema.loop");
    loop_null: "loop" / "null", |t| replace_once(t, block(t, "loop"), "    loop: null\n") => Expect::Null("$schema.loop");
    loop_wrong_type: "loop" / "wrong_type", |t| replace_once(t, block(t, "loop"), "    loop: string\n") => Expect::WrongType("$schema.loop");
    loop_one_wrong: "loop" / "one_wrong", |t| replace_once(t, "last_exit_code: \"number(integer)", "last_exit_code: \"number(integr)") => Expect::Resolve("$schema.loop");
    loop_every_wrong: "loop" / "every_wrong", |t| edit_members(t, "loop", every_member_wrong) => Expect::Resolve("$schema.loop");
    loop_empty: "loop" / "empty", |t| replace_once(t, block(t, "loop"), "    loop: \"\"\n") => Expect::Empty("$schema.loop");
    loop_duplicate: "loop" / "duplicate", |t| edit_members(t, "loop", |line| if line.contains("is_last:") { format!("{line}\n{line}") } else { line.to_string() }) => Expect::Yaml;
    loop_trailing: "loop" / "trailing", |t| replace_once(t, block(t, "loop"), "    loop: [object\n") => Expect::Yaml;
    // ── group ──
    group_control: "group" / "control", |t| t.to_string() => Expect::Loads;
    group_absent: "group" / "absent", |t| replace_once(t, block(t, "group"), "") => Expect::Missing("$schema.group");
    group_null: "group" / "null", |t| replace_once(t, block(t, "group"), "    group: null\n") => Expect::Null("$schema.group");
    group_wrong_type: "group" / "wrong_type", |t| replace_once(t, block(t, "group"), "    group: true\n") => Expect::WrongType("$schema.group");
    group_one_wrong: "group" / "one_wrong", |t| replace_once(t, "outcome: \"enum(succeeded, failed, cancelled, skipped)", "outcome: \"enum(succeeded, failed, cancelled, skipped; bogus())") => Expect::Resolve("$schema.group");
    group_every_wrong: "group" / "every_wrong", |t| replace_once(t, block(t, "group"), "    group:\n        legs: strng\n        succeeded: strng\n        failed: strng\n        cancelled: strng\n        \"<string>\": strng\n") => Expect::Resolve("$schema.group");
    group_empty: "group" / "empty", |t| replace_once(t, block(t, "group"), "    group: {}\n") => Expect::Empty("$schema.group");
    group_duplicate: "group" / "duplicate", |t| edit_members(t, "group", |line| if line.contains("failed: \"string[]") { format!("{line}\n{line}") } else { line.to_string() }) => Expect::Yaml;
    group_trailing: "group" / "trailing", |t| format!("{t}        ]: [\n") => Expect::Yaml;
}

/// Claudine extends the expression language with no host function: no file in
/// `claudine/schemas/` is a function catalog, and no `.facts.json` artifact
/// sits there. A host function needs both, registered together: its
/// `function(...)` declaration in a catalog there, its Rust registration
/// through `darkmatter::markdown::compose::expression::host::HostFunctions`,
/// and the artifact that registration writes
/// (`HostFunctions::facts_artifact`), kept fresh by a test comparing the two
/// byte for byte. This test fails when a catalog or an artifact appears
/// without that wiring. The host lifecycle artifact beside `globals.yaml`
/// (`globals.lifecycle.json`) is a different artifact, about events rather
/// than functions; [`lifecycle_artifact`] keeps it fresh.
#[test]
fn claudine_ships_no_host_function_catalog_and_no_facts_artifact() {
    let schemas = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../schemas");
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
    use claudine::composition::lifecycle::{lifecycle_artifact, reject_incompatible_calls};
    use claudine::composition::parse_lifecycle_config;
    use claudine::composition::schema::globals::{HOST_CATALOG_PATH, HOST_CATALOG_TEXT, claudine_globals};
    use darkmatter::lifecycle::UnknownVerbPolicy;
    use darkmatter::lifecycle::host_profile::HostLifecycleProfile;
    use darkmatter::markdown::Markdown;
    use darkmatter::markdown::compose::expression::host::FactsArtifact;
    use darkmatter::markdown::schemas::DarkmatterSchemas;

    /// The committed artifact DMLS reads beside `globals.yaml`.
    const COMMITTED: &str = include_str!("../../../schemas/globals.lifecycle.json");

    fn profile() -> HostLifecycleProfile {
        HostLifecycleProfile::from_artifact(
            claudine_globals(),
            Path::new(HOST_CATALOG_PATH),
            HOST_CATALOG_TEXT,
            FactsArtifact::Text(COMMITTED),
        )
        .expect("the committed artifact reads")
    }

    /// The freshness check: Claudine's registration and parse options, written
    /// now, are the committed bytes.
    #[test]
    fn the_committed_lifecycle_artifact_is_fresh() {
        assert!(
            lifecycle_artifact() == COMMITTED,
            "claudine/schemas/globals.lifecycle.json is stale; run `just regen-lifecycle-artifact` in claudine/"
        );
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
