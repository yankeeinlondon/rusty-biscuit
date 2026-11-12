//! Darkmatter's shipped artifact (`darkmatter::markdown::schemas::shipped`):
//! the checked-in prepared form of its compiled-in schema sources.
//!
//! The reader under test is Darkmatter's payload loader
//! ([`Registry::load_over`] over the shared `simplified_schema::shipped`
//! decoder, whose document and header matrices simplified-schema's suite
//! covers). Its default outcome for an invalid artifact is a refusal of the
//! whole artifact with a `FallbackReason` and a JSON pointer; no entry of it
//! is exposed, and each consumer prepares its item from the compiled text,
//! with results equal to the accepted artifact's. Listed exception: a `null`
//! catalog `description` is a valid record (`None`), while an absent one is
//! malformed.
//!
//! Every row edits one member of the checked-in artifact (which
//! `shipped_artifact_is_fresh` proves current) in memory; nothing here writes
//! a repository file or a production registry.

use std::sync::OnceLock;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::FunctionCatalog;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::catalog::{DARKMATTER_GLOBALS, GlobalCatalog};
use darkmatter::markdown::schemas::shipped::{self, DarkmatterPayload, FreshDarkmatter, Registry};
use serde_json::{Value, json};
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::shipped::{self as mechanism, DependencyFingerprint, FallbackReason, FreshLower, LowerArtifact};

use crate::request_support::context_at;

#[path = "../../examples/shipped_schemas.rs"]
#[allow(dead_code)]
mod generator;

/// The checked-in artifact, as the library embeds it.
const CHECKED_IN: &[u8] = include_bytes!("../../generated/shipped.json");

/// One fresh, uncached generation of both artifacts per test process.
fn fresh() -> &'static (FreshLower, FreshDarkmatter) {
    static FRESH: OnceLock<(FreshLower, FreshDarkmatter)> = OnceLock::new();
    FRESH.get_or_init(|| {
        let lower = FreshLower::prepare().expect("the library's sources generate");
        let darkmatter = FreshDarkmatter::prepare(&lower).expect("Darkmatter's sources generate");
        (lower, darkmatter)
    })
}

/// The process's current Simplified Schema artifact as a dependency.
fn current_lower() -> LowerArtifact {
    simplified_schema::shipped::registry::registry().as_dependency()
}

fn checked_in_value() -> Value {
    serde_json::from_slice(CHECKED_IN).expect("the artifact is JSON")
}

/// `value` in the artifact layout: the header pretty-printed, each payload
/// member compact on its own line.
fn encode(value: &Value) -> Vec<u8> {
    let mut text = format!("{{\n\"header\": {},\n\"payload\": {{\n", serde_json::to_string_pretty(&value["header"]).unwrap());
    let members: Vec<String> = value["payload"]
        .as_object()
        .expect("an object payload")
        .iter()
        .map(|(key, member)| format!("{}:{}", serde_json::to_string(key).unwrap(), serde_json::to_string(member).unwrap()))
        .collect();
    text.push_str(&members.join(",\n"));
    text.push_str("\n}\n}\n");
    text.into_bytes()
}

fn decoded_checked_in() -> DarkmatterPayload {
    serde_json::from_value(checked_in_value()["payload"].clone()).expect("the payload decodes")
}

/// A document whose frontmatter breaks the baseline (`title` is a string)
/// and a `timing` typed through the reserved namespace, as the problems a
/// validation reports.
fn validation_problems(attach: impl FnOnce(DarkmatterSchemas) -> DarkmatterSchemas) -> Vec<(String, String)> {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let schemas = attach(DarkmatterSchemas::new(context_at(&root)));
    let markdown: Markdown =
        "---\n$schema:\n  t: timing@darkmatter:timing\ntitle: 5\nt:\n  document_ms: soon\n---\nBody\n".into();
    let report = schemas.validate(&markdown).expect("the schema prepares");
    let mut problems: Vec<(String, String)> =
        report.problems.iter().map(|problem| (problem.path.clone(), problem.message.clone())).collect();
    problems.sort();
    problems
}

// ── Freshness and round trip ────────────────────────────────────────────────

#[test]
fn shipped_artifact_is_fresh() {
    // Freshly prepares the library's envelope definitions and every
    // Darkmatter source, uncached, and compares the encoding byte for byte.
    // The other packages' inputs are spelled for CI's test-input index, so a
    // change to them (which regenerates the lower artifact) runs this test.
    let _ = include_bytes!("../../../../simplified-schema/lib/generated/shipped.json");
    let _ = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../../simplified-schema/schemas/schema.yaml");
    let _ = include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml");
    // The library code that parses, admits, resolves, lowers, and encodes the
    // shipped schemas (declared as `source-inputs` in Darkmatter's manifest).
    let _ = include_str!("../../../../simplified-schema/lib/src/embedded.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/envelope.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/resolve.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/resolve/graph.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/shipped/mod.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/shipped/records.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/shipped/wire.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/shipped/wire_support.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/convert.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/definitions.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/grammar.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/mod.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/standalone.rs");
    let _ = include_str!("../../../../simplified-schema/lib/src/simplified/types.rs");
    let checked_in = include_bytes!("../../generated/shipped.json");
    let fresh = shipped::generate().expect("fresh generation");
    if let Err(stale) = mechanism::check_fresh(checked_in, &fresh, shipped::ARTIFACT_PATH, shipped::REGENERATE_COMMAND) {
        panic!("{stale}");
    }
}

#[test]
fn a_mutated_source_fails_freshness_and_acceptance_naming_the_command() {
    let mutated: Vec<EmbeddedDocument<'static>> = shipped::sources()
        .iter()
        .map(|document| EmbeddedDocument {
            path: document.path,
            text: if document.path == "darkmatter/schemas/partials/timing.yaml" {
                Box::leak(format!("{}\n# edited\n", document.text).into_boxed_str())
            } else {
                document.text
            },
        })
        .collect();
    let lower = [fresh().0.as_dependency()];
    let descriptor = shipped::descriptor_over(&mutated, &lower);
    let regenerated = mechanism::generate(&descriptor, &decoded_checked_in()).unwrap();
    let stale = mechanism::check_fresh(CHECKED_IN, &regenerated, shipped::ARTIFACT_PATH, shipped::REGENERATE_COMMAND)
        .expect_err("an edited source changes the header");
    assert!(stale.to_string().contains("just shipped-schemas"), "{stale}");
    let refusal = mechanism::accept::<DarkmatterPayload, _>(&descriptor, Some(CHECKED_IN), |_, payload| Ok(payload))
        .expect_err("the checked-in artifact no longer describes the edited text");
    assert_eq!(refusal.reason, FallbackReason::SourceMismatch);
    assert!(refusal.pointer.as_deref().is_some_and(|pointer| pointer.ends_with("/digest")), "{refusal}");
}

#[test]
fn the_checked_in_artifact_round_trips_to_uncached_preparation_in_order() {
    let lower = [current_lower()];
    let accepted = mechanism::accept::<DarkmatterPayload, _>(
        &shipped::descriptor_over(shipped::sources(), &lower),
        Some(CHECKED_IN),
        |_, payload| Ok(payload),
    )
    .expect("the checked-in artifact is accepted");
    let uncached = shipped::prepare_payload(&fresh().0).expect("uncached preparation");
    assert_eq!(accepted.payload(), &uncached);
    // `Value` and map equality ignore order; the encoding does not.
    assert_eq!(serde_json::to_string(accepted.payload()).unwrap(), serde_json::to_string(&uncached).unwrap());

    // Ordered metadata as consumers observe it, through the accepted views.
    let registry = Registry::load(Some(CHECKED_IN));
    assert!(registry.refusal().is_none(), "{:?}", registry.refusal());
    let globals: Vec<&str> = registry.global_catalog().globals().map(|global| global.name()).collect();
    assert_eq!(globals, ["fm", "ctx", "current", "env", "err", "timing"]);
    assert_eq!(
        registry.global_catalog(),
        &GlobalCatalog::from_record(uncached.catalog.clone(), DARKMATTER_GLOBALS).unwrap()
    );
    let order = |catalog: &FunctionCatalog| -> Vec<(String, String, usize, usize)> {
        catalog
            .descriptors
            .iter()
            .map(|descriptor| (descriptor.category.to_string(), descriptor.name.to_string(), descriptor.order, descriptor.overload))
            .collect()
    };
    assert_eq!(order(registry.function_catalog()), order(&uncached.functions));
    assert_eq!(registry.function_catalog().categories, uncached.functions.categories);
    assert!(
        registry.function_catalog().named_type_names().eq(uncached.functions.named_type_names()),
        "named helper types keep authored order"
    );
    assert_eq!(registry.effect_catalog(), &uncached.effects);
    let fm = registry.global_catalog().global("fm").unwrap();
    let uncached_fm = &uncached.catalog.globals[0];
    assert!(fm.definitions().iter().map(|(key, _)| key).eq(uncached_fm.definitions.iter().map(|(key, _)| key)));
    let tables: Vec<&str> = uncached.named_types.tables.iter().map(|table| table.identity.as_str()).collect();
    assert_eq!(tables.first(), Some(&"darkmatter/schemas/partials/fm.yaml"), "tables follow the source table");
    assert_eq!(registry.inline_baseline(), &uncached.inline_baseline);
    assert_eq!(registry.baseline_json_schema(), &uncached.baseline.json_schema);
    assert_eq!(
        uncached.baseline.digest,
        mechanism::CanonicalDigestRecord::of(&uncached.baseline.json_schema),
        "the baseline's recorded digest is the canonical digest of its lowered schema"
    );
}

#[test]
fn fresh_generation_is_deterministic_and_ignores_checked_in_artifacts() {
    let (lower, darkmatter) = fresh();
    assert_eq!(darkmatter.artifact(), FreshDarkmatter::prepare(lower).unwrap().artifact());
    assert_eq!(darkmatter.artifact().last(), Some(&b'\n'));
    assert!(!darkmatter.artifact().ends_with(b"\n\n"));
    assert_eq!(darkmatter.as_dependency(), Registry::load(Some(CHECKED_IN)).as_dependency());
}

// ── Accepted lookups ────────────────────────────────────────────────────────

/// A cold process reads every shipped item from the accepted artifacts:
/// nothing is prepared from text and the baseline check does not run.
#[cfg(feature = "work-counters")]
#[test]
fn cold_accepted_lookups_prepare_nothing_and_skip_the_baseline_check() {
    use darkmatter::work_counters::{Work, snapshot};
    let before = snapshot();
    assert!(shipped::registry().refusal().is_none(), "{:?}", shipped::registry().refusal());
    let _ = darkmatter::markdown::schemas::darkmatter_base_json_schema_ref();
    let _ = darkmatter::markdown::schemas::darkmatter_base_schema();
    let _ = GlobalCatalog::darkmatter();
    let kinds = darkmatter::markdown::schemas::KindRegistry::shipped();
    assert!(kinds.claim("schema-trigger").is_some());
    assert!(!darkmatter::markdown::compose::expression::expression_function_catalog().descriptors.is_empty());
    assert!(!darkmatter::effects::effect_function_catalog().descriptors.is_empty());
    let problems = validation_problems(|schemas| schemas.with_darkmatter_baseline_json_schema().expect("attaches"));
    assert!(problems.iter().any(|(path, _)| path == "/t/document_ms"), "the reserved namespace typed `t`: {problems:?}");
    let work = snapshot().since(&before);
    assert_eq!(work.get(Work::ShippedRuntimePreparation), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::BaselineCheck), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::ArtifactAccept), 2, "the library's and Darkmatter's artifacts: {:?}", work.nonzero());
}

// ── Fallback: a refused artifact prepares each item from text ───────────────

/// Loads `bytes` over `lower`, checks the refusal (of the whole artifact,
/// or of the section a consumer reads), and checks that every fallback view
/// equals the accepted artifact's and that a validation through the
/// fallback baseline runs the baseline check and reports the same problems.
#[track_caller]
fn assert_falls_back(bytes: Option<&[u8]>, lower: LowerArtifact, reason: FallbackReason, pointer: &str) {
    let refused = Registry::load_over(bytes, lower);
    let refusal = refused.check_all().expect_err("the artifact or a section is refused");
    assert_eq!(refusal.reason, reason, "{refusal}");
    assert!(refusal.pointer.as_deref().unwrap_or("").starts_with(pointer), "pointer is not under `{pointer}`: {refusal}");
    if refused.refusal().is_some() {
        assert!(refused.named_types().is_none(), "no entry of a refused artifact is served");
        assert_eq!(refused.as_dependency(), LowerArtifact::Refused { owner: shipped::OWNER.into() });
    }

    let accepted = Registry::load(Some(CHECKED_IN));
    assert_eq!(refused.global_catalog(), accepted.global_catalog());
    assert_eq!(refused.baseline_json_schema(), accepted.baseline_json_schema());
    assert_eq!(refused.inline_baseline(), accepted.inline_baseline());
    assert_eq!(refused.function_catalog(), accepted.function_catalog());
    assert_eq!(refused.effect_catalog(), accepted.effect_catalog());
    let claims = |registry: &Registry| -> Vec<(String, Vec<String>)> {
        registry
            .shipped_kind_registry()
            .claims()
            .map(|(kind, claim)| match claim {
                darkmatter::markdown::schemas::yaml_documents::Claim::Healthy(definition) => {
                    (kind.to_string(), vec![definition.resolved().json_schema.to_string()])
                }
                other => panic!("a shipped claim is healthy: {other:?}"),
            })
            .collect()
    };
    assert_eq!(claims(&refused), claims(&accepted));

    #[cfg(feature = "work-counters")]
    let before = darkmatter::work_counters::snapshot();
    let fallback = validation_problems(|schemas| schemas.with_darkmatter_baseline_in(&refused).expect("attaches"));
    // Only a baseline prepared from text is checked again; a refused section
    // other than the catalog's or the baseline's keeps the accepted one.
    #[cfg(feature = "work-counters")]
    assert_eq!(
        darkmatter::work_counters::snapshot().since(&before).get(darkmatter::work_counters::Work::BaselineCheck),
        u64::from(!refused.baseline_is_accepted())
    );
    let ordinary = validation_problems(|schemas| schemas.with_darkmatter_baseline_in(&accepted).expect("attaches"));
    assert!(!ordinary.is_empty());
    assert_eq!(fallback, ordinary);
}

fn edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = checked_in_value();
    edit(&mut value);
    encode(&value)
}

#[test]
fn missing_bytes_fall_back() {
    assert_falls_back(None, current_lower(), FallbackReason::MissingArtifact, "");
}

#[test]
fn malformed_json_falls_back() {
    assert_falls_back(Some(&CHECKED_IN[..CHECKED_IN.len() / 2]), current_lower(), FallbackReason::MalformedArtifact, "");
}

#[test]
fn a_source_mismatch_falls_back() {
    let bytes = edited(|value| value["header"]["sources"][0]["digest"] = json!("0000000000000000"));
    assert_falls_back(Some(&bytes), current_lower(), FallbackReason::SourceMismatch, "/header/sources/0/digest");
}

#[test]
fn another_preparation_revision_falls_back() {
    let bytes = edited(|value| value["header"]["preparation_revision"] = json!(shipped::PREPARATION_REVISION + 1));
    assert_falls_back(Some(&bytes), current_lower(), FallbackReason::PreparationRevisionMismatch, "/header/preparation_revision");
}

#[test]
fn a_malformed_record_falls_back() {
    let bytes = edited(|value| value["payload"]["functions"]["descriptors"][0]["order"] = json!("first"));
    assert_falls_back(Some(&bytes), current_lower(), FallbackReason::MalformedArtifact, "/payload/functions/descriptors/0/order");
    let registry = Registry::load(Some(&bytes));
    assert!(registry.refusal().is_none() && registry.baseline_is_accepted(), "only the functions section is refused");
}

#[test]
fn a_refused_catalog_section_refuses_the_baseline_typed_by_it() {
    let bytes = edited(|value| value["payload"]["catalog"]["globals"][0]["name"] = json!(5));
    assert_falls_back(Some(&bytes), current_lower(), FallbackReason::MalformedArtifact, "/payload/catalog/globals/0/name");
    assert!(!Registry::load(Some(&bytes)).baseline_is_accepted(), "the baseline falls back with its catalog");
}

#[cfg(feature = "work-counters")]
#[test]
fn reading_the_baseline_decodes_no_unrelated_section() {
    use darkmatter::work_counters::{Work, snapshot};
    // A broken functions section is never read when only the baseline is.
    let bytes = edited(|value| value["payload"]["functions"] = json!("broken"));
    let registry = Registry::load(Some(&bytes));
    let before = snapshot();
    assert!(registry.baseline_is_accepted());
    let _ = registry.global_catalog();
    assert_eq!(snapshot().since(&before).get(Work::ArtifactFallbackMalformed), 0, "the functions section was not decoded");
    let _ = registry.function_catalog();
    assert_eq!(snapshot().since(&before).get(Work::ArtifactFallbackMalformed), 1, "reading it refuses it once");
}

#[test]
fn an_unusable_record_falls_back() {
    let bytes = edited(|value| value["payload"]["catalog"]["globals"][1]["name"] = json!("fm"));
    assert_falls_back(Some(&bytes), current_lower(), FallbackReason::UnusableRecord, "/payload/catalog/globals/1/name");
}

#[test]
fn a_stale_lower_artifact_refuses_darkmatter_even_when_its_bytes_match_the_header() {
    // The lower artifact is stale (an older revision), and Darkmatter's
    // artifact was generated over exactly those stale bytes, so the header
    // digests agree; the lower refusal alone refuses it.
    use simplified_schema::shipped::registry as lower;
    let mut stale: Value = serde_json::from_slice(lower::ARTIFACT).unwrap();
    stale["header"]["preparation_revision"] = json!(lower::PREPARATION_REVISION + 1);
    let stale = encode(&stale);
    let over_stale = [LowerArtifact::Current(DependencyFingerprint::of_bytes(lower::OWNER, lower::PREPARATION_REVISION, &stale))];
    let generated = mechanism::generate(&shipped::descriptor_over(shipped::sources(), &over_stale), &decoded_checked_in()).unwrap();
    let stale_lower = lower::Registry::load(Some(&stale));
    assert_eq!(stale_lower.refusal().map(|refusal| refusal.reason), Some(FallbackReason::PreparationRevisionMismatch));
    assert_falls_back(Some(&generated), stale_lower.as_dependency(), FallbackReason::DependencyMismatch, "/header/dependencies");
}

#[test]
fn a_custom_shipped_sources_under_darkmatter_s_paths_is_parsed_not_served() {
    // The same namespace, folder, and paths as Darkmatter's, with another
    // `timing.yaml`: the attached artifact tables describe other text, so
    // the resolver parses this one.
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/timing.yaml",
            text: "kind: schema\ntypes:\n  timing:\n    document_ms: string\n",
        },
        EmbeddedDocument { path: "host/main.yaml", text: "kind: schema\n$schema:\n  t: timing@darkmatter:timing\n" },
    ];
    assert!(shipped::registry().named_types().is_some(), "the process artifact is accepted");
    let custom = ShippedSources::new("darkmatter:", "darkmatter/schemas/partials", DOCUMENTS).with_prepared(&shipped::NAMED_TYPES);
    let resolved = simplified_schema::resolve::prepare_embedded_schema("host/main.yaml", DOCUMENTS, custom).expect("resolves");
    let member = resolved.json_schema["properties"]["t"].to_string();
    assert!(member.contains("\"string\"") && !member.contains("\"integer\""), "the custom text typed `t`: {member}");
}

// ── Payload record validators ───────────────────────────────────────────────

/// Loads one edited checked-in artifact and checks its refusal.
#[track_caller]
fn assert_record_refused(bytes: Vec<u8>, reason: FallbackReason, pointer: &str) {
    let refusal = Registry::load(Some(&bytes)).check_all().expect_err("the artifact or a section is refused");
    assert_eq!(refusal.reason, reason, "{refusal}");
    assert!(refusal.pointer.as_deref().unwrap_or("").starts_with(pointer), "pointer is not under `{pointer}`: {refusal}");
}

macro_rules! refused {
    ($($name:ident: $edit:expr => $reason:ident at $pointer:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_record_refused(edited($edit), FallbackReason::$reason, $pointer);
        }
    )*};
}

fn remove(value: &mut Value, pointer: &str) {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    value.pointer_mut(parent).unwrap().as_object_mut().unwrap().shift_remove(key);
}

#[test]
fn the_re_encoded_checked_in_artifact_is_the_accepted_control() {
    assert_eq!(edited(|_| {}), CHECKED_IN, "re-encoding is byte-identical");
    assert_eq!(Registry::load(Some(CHECKED_IN)).check_all(), Ok(()));
}

#[test]
fn a_null_catalog_description_is_none_and_an_absent_one_is_malformed() {
    let null = Registry::load(Some(&edited(|value| value["payload"]["catalog"]["description"] = Value::Null)));
    assert_eq!(null.check_all(), Ok(()));
    assert_eq!(null.global_catalog().description(), None);
    assert_record_refused(
        edited(|value| remove(value, "/payload/catalog/description")),
        FallbackReason::MalformedArtifact,
        "/payload/catalog",
    );
}

refused! {
    // A payload member: required, of its type.
    an_absent_payload_member: |value| remove(value, "/payload/effects") => UnusableRecord at "/payload/effects";
    an_unknown_payload_member: |value| value["payload"]["extra"] = json!(1) => UnusableRecord at "/payload/extra";
    a_baseline_that_is_not_a_schema_object: |value| value["payload"]["baseline"]["json_schema"] = json!(5) => UnusableRecord at "/payload/baseline/json_schema";
    a_baseline_record_of_the_wrong_type: |value| value["payload"]["baseline"] = json!(5) => MalformedArtifact at "/payload/baseline";

    // The baseline's digest: computed under this process's canonical revision, of its type.
    a_baseline_digest_of_another_revision: |value| value["payload"]["baseline"]["digest"]["revision"] = json!(99) => UnusableRecord at "/payload/baseline/digest/revision";
    an_absent_baseline_digest: |value| remove(value, "/payload/baseline/digest") => MalformedArtifact at "/payload/baseline";
    a_null_baseline_digest: |value| value["payload"]["baseline"]["digest"] = Value::Null => MalformedArtifact at "/payload/baseline";
    a_short_baseline_digest: |value| value["payload"]["baseline"]["digest"]["digest"] = json!("9ad0") => MalformedArtifact at "/payload/baseline";
    a_baseline_classification_of_the_wrong_type: |value| value["payload"]["baseline"]["digest"]["judges_files"] = json!("no") => MalformedArtifact at "/payload/baseline";
    kind_definitions_of_the_wrong_type: |value| value["payload"]["kind_definitions"] = json!({}) => MalformedArtifact at "/payload/kind_definitions";

    // Named-type tables: only the shipped table's documents, once each.
    a_table_outside_the_inventory: |value| value["payload"]["named_types"]["tables"][0]["identity"] = json!("darkmatter/schemas/partials/other.yaml") => UnusableRecord at "/payload/named_types/tables/0";
    a_repeated_table: |value| { let first = value["payload"]["named_types"]["tables"][0].clone(); value["payload"]["named_types"]["tables"].as_array_mut().unwrap().push(first); } => UnusableRecord at "/payload/named_types/tables";
    an_inventory_that_is_not_the_shipped_table: |value| { value["payload"]["named_types"]["inventory"].as_array_mut().unwrap().pop(); } => UnusableRecord at "/payload/named_types/inventory";

    // The global catalog: its own path, exactly the required globals.
    a_catalog_at_another_path: |value| value["payload"]["catalog"]["identity"] = json!("darkmatter/schemas/other.yaml") => UnusableRecord at "/payload/catalog/identity";
    a_missing_global: |value| { value["payload"]["catalog"]["globals"].as_array_mut().unwrap().pop(); } => UnusableRecord at "/payload/catalog/globals";
    a_global_of_the_wrong_kind: |value| value["payload"]["catalog"]["globals"][0]["definition"] = json!({"Single": {"ty": {"Primitive": "String"}, "is_array": false, "constraints": [], "array_constraints": [], "description": null}}) => UnusableRecord at "/payload/catalog/globals/0";

    // Kind definitions: one per compiled definition, in table order.
    a_repeated_kind_definition: |value| { let first = value["payload"]["kind_definitions"][0].clone(); value["payload"]["kind_definitions"].as_array_mut().unwrap().insert(1, first); } => UnusableRecord at "/payload/kind_definitions";
    swapped_kind_definitions: |value| value["payload"]["kind_definitions"].as_array_mut().unwrap().swap(0, 1) => UnusableRecord at "/payload/kind_definitions/0/identity";
    a_kind_definition_claiming_nothing: |value| value["payload"]["kind_definitions"][2]["kinds"] = json!([]) => UnusableRecord at "/payload/kind_definitions/2/kinds";
    a_host_file_origin: |value| value["payload"]["kind_definitions"][0]["origin"]["source"] = json!({"File": "/home/user/schema.yaml"}) => MalformedArtifact at "/payload/kind_definitions/0/origin/source";

    // Function catalogs: declared categories, adjacent overloads, positions.
    an_undeclared_category: |value| value["payload"]["functions"]["descriptors"][0]["category"] = json!("Nowhere") => UnusableRecord at "/payload/functions/descriptors/0/category";
    a_misnumbered_overload: |value| value["payload"]["effects"]["descriptors"][1]["overload"] = json!(5) => UnusableRecord at "/payload/effects/descriptors/1/overload";
    a_misplaced_category_position: |value| value["payload"]["functions"]["descriptors"][1]["order"] = json!(99) => UnusableRecord at "/payload/functions/descriptors/1/order";
    an_absent_example_reason: |value| {
        let descriptors = value["payload"]["functions"]["descriptors"].as_array_mut().unwrap();
        let example = descriptors.iter_mut().flat_map(|descriptor| descriptor["examples"].as_array_mut().unwrap().iter_mut()).find_map(|example| example.get_mut("Call")).expect("an inline example");
        example.as_object_mut().unwrap().shift_remove("display_only");
    } => MalformedArtifact at "/payload/functions/descriptors";
}

// ── The generator command ───────────────────────────────────────────────────

fn run_generator(
    arguments: &[&str],
    artifact: &std::path::Path,
    compiled_lower: &[u8],
    generate: impl FnOnce() -> Result<generator::Generated, String>,
) -> (u8, Vec<u8>, String) {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code =
        generator::run(arguments.iter().map(|argument| argument.to_string()), artifact, compiled_lower, generate, &mut stdout, &mut stderr);
    (code, stdout, String::from_utf8(stderr).unwrap())
}

fn generated() -> Result<generator::Generated, String> {
    let (lower, darkmatter) = fresh();
    Ok(generator::Generated { lower: lower.artifact().to_vec(), artifact: darkmatter.artifact().to_vec() })
}

#[test]
fn the_generator_checks_emits_and_writes() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    let lower = simplified_schema::shipped::registry::ARTIFACT;
    std::fs::write(&artifact, CHECKED_IN).unwrap();
    assert_eq!(run_generator(&["--check"], &artifact, lower, generated), (0, Vec::new(), String::new()));
    let (code, stdout, _) = run_generator(&[], &artifact, b"stale lower", generated);
    assert_eq!((code, stdout.as_slice()), (0, fresh().1.artifact()), "emitting checks no lower artifact");
    std::fs::write(&artifact, b"stale").unwrap();
    assert_eq!(run_generator(&["--write"], &artifact, lower, generated).0, 0);
    assert_eq!(std::fs::read(&artifact).unwrap(), fresh().1.artifact());
    assert_eq!(run_generator(&["--check", "--write"], &artifact, lower, generated).0, 2);
}

#[test]
fn a_stale_lower_artifact_stops_check_and_write_naming_the_root_command() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, b"previous").unwrap();
    for mode in ["--check", "--write"] {
        let (code, stdout, stderr) = run_generator(&[mode], &artifact, b"stale lower", generated);
        assert_eq!(code, 1, "{mode}");
        assert!(stdout.is_empty());
        assert!(stderr.contains("simplified-schema/lib/generated/shipped.json") && stderr.contains("just shipped-schemas"), "{stderr}");
    }
    assert_eq!(std::fs::read(&artifact).unwrap(), b"previous");
}

#[test]
fn the_generator_reports_where_a_malformed_artifact_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let supplied = directory.path().join("supplied.json");
    let malformed = edited(|value| value["payload"]["catalog"]["identity"] = json!(5));
    std::fs::write(&supplied, &malformed).unwrap();
    let (code, stdout, stderr) = run_generator(
        &["--check", "--artifact", supplied.to_str().unwrap()],
        &directory.path().join("shipped.json"),
        simplified_schema::shipped::registry::ARTIFACT,
        generated,
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(
        stderr.contains("/payload/catalog/identity") && stderr.contains("line") && stderr.contains("just shipped-schemas"),
        "{stderr}"
    );
    assert_eq!(std::fs::read(&supplied).unwrap(), malformed, "a check writes nothing");
}

#[test]
fn a_failed_generation_leaves_the_artifact_in_place() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, b"previous").unwrap();
    let lower = simplified_schema::shipped::registry::ARTIFACT;
    let (code, _, stderr) = run_generator(&["--write"], &artifact, lower, || Err("a catalog failed".into()));
    assert_eq!(code, 1);
    assert!(stderr.contains("a catalog failed"), "{stderr}");
    let refused = || Ok(generator::Generated { lower: lower.to_vec(), artifact: b"{\"not\": \"an artifact\"}\n".to_vec() });
    assert_eq!(run_generator(&["--write"], &artifact, lower, refused).0, 1);
    assert_eq!(std::fs::read(&artifact).unwrap(), b"previous");
    assert!(!directory.path().join("shipped.json.tmp").exists());
}
