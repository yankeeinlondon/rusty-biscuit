//! Claudine's shipped artifact (`claudine::composition::schema::shipped`):
//! the checked-in prepared form of its trigger bundle and host catalog.
//!
//! The reader under test is Claudine's payload loader ([`Registry::load_over`]
//! over the shared `simplified_schema::shipped` decoder, whose document and
//! header matrices simplified-schema's suite covers). An invalid artifact is
//! refused with a `FallbackReason` and a JSON pointer; no entry of it is
//! served, and each consumer prepares from the compiled text with results
//! equal to the accepted artifact's. Listed exception: a `null` catalog
//! `description` is a valid record (`None`), while an absent one is
//! malformed.
//!
//! Every row edits the checked-in artifact (which `shipped_artifact_is_fresh`
//! proves current) in memory; nothing here writes a repository file or a
//! production registry. Every schema context is built inside a temporary
//! directory, never the process CWD.

use std::collections::HashMap;
use std::sync::OnceLock;

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use claudine::composition::schema::bundle::CLAUDINE_SCHEMA_BUNDLE_NAME;
use claudine::composition::schema::shipped::{self, ClaudinePayload, FreshClaudine, Registry};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::catalog::GlobalCatalog;
use darkmatter::markdown::schemas::shipped::FreshDarkmatter;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, EmbeddedSchemaBundle};
use serde_json::{Value, json};
use simplified_schema::embedded::EmbeddedDocument;
use simplified_schema::shipped::{self as mechanism, DependencyFingerprint, FallbackReason, FreshLower, LowerArtifact};

#[path = "../../examples/shipped_schemas.rs"]
#[allow(dead_code)]
mod generator;

/// The checked-in artifact, as the library embeds it.
const CHECKED_IN: &[u8] = include_bytes!("../../generated/shipped.json");

/// One fresh, uncached generation of all three artifacts per test process.
fn fresh() -> &'static (FreshLower, FreshDarkmatter, FreshClaudine) {
    static FRESH: OnceLock<(FreshLower, FreshDarkmatter, FreshClaudine)> = OnceLock::new();
    FRESH.get_or_init(|| {
        let lower = FreshLower::prepare().expect("the library's sources generate");
        let darkmatter = FreshDarkmatter::prepare(&lower).expect("Darkmatter's sources generate");
        let claudine = FreshClaudine::prepare(&lower, &darkmatter).expect("Claudine's sources generate");
        (lower, darkmatter, claudine)
    })
}

/// The process's current lower artifacts as dependencies, in
/// [`Registry::load_over`] order.
fn current_lowers() -> [LowerArtifact; 2] {
    [
        simplified_schema::shipped::registry::registry().as_dependency(),
        darkmatter::markdown::schemas::shipped::registry().as_dependency(),
    ]
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

fn edited(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut value = checked_in_value();
    edit(&mut value);
    encode(&value)
}

fn decoded_checked_in() -> ClaudinePayload {
    serde_json::from_value(checked_in_value()["payload"].clone()).expect("the payload decodes")
}

/// A resolution context inside a temporary directory, with a fixture home
/// and an empty environment: no disk schema root exists.
struct Bare {
    _dir: tempfile::TempDir,
    context: FileResolutionContext,
}

impl Bare {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("work")).expect("work dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        let snapshot =
            RequestSnapshot::new(root.join("work")).with_home(Some(root.join("home"))).with_env(HashMap::new());
        let context = build_resolution_context(&snapshot).expect("fixture context");
        Self { _dir: dir, context }
    }

    /// The effective schema of an in-memory document holding `frontmatter`,
    /// with `bundle` as the only trigger source.
    fn effective(&self, bundle: EmbeddedSchemaBundle, frontmatter: &str) -> EffectiveSchema {
        let schemas = DarkmatterSchemas::new(self.context.clone())
            .with_embedded_trigger_bundle(bundle)
            .and_then(DarkmatterSchemas::with_trigger_discovery)
            .expect("the bundle loads");
        let markdown = Markdown::from(format!("---\n{frontmatter}\n---\nbody\n").as_str());
        schemas.effective_for(&markdown).expect("resolves").expect("a trigger applies")
    }
}

// ── Freshness and round trip ────────────────────────────────────────────────

#[test]
fn shipped_artifact_is_fresh() {
    // Freshly prepares both lower artifacts and every Claudine source,
    // uncached, and compares the encoding byte for byte. The lower artifacts
    // are spelled for CI's test-input index, so regenerating either runs this
    // test.
    let _ = include_bytes!("../../../../simplified-schema/lib/generated/shipped.json");
    let _ = include_bytes!("../../../../darkmatter/lib/generated/shipped.json");
    // Every source the lower artifacts fingerprint: a lower text edit
    // changes Claudine's dependency digests even before it is regenerated.
    let _ = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/darkmatter.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/context-variables.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/error.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/fm.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/function-examples/as_line_separated.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/function-examples/as_ordered_list.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/function-examples/as_tsv.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/function-examples/as_unordered_list.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/functions.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/lifecycle.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/md-lifecycle.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/mutations.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/style.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/partials/timing.yaml");
    let _ = include_str!("../../../../darkmatter/schemas/schema-trigger.yaml");
    let _ = include_str!("../../../../simplified-schema/schemas/schema.yaml");
    let _ = include_str!("../../../../simplified-schema/schemas/yaml-schema.yaml");
    // The lower code that prepares Claudine's bundle and host catalog
    // (declared as `source-inputs` in Claudine's manifest).
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/catalog.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/embedded.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/resolve.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/shipped.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/triggers/assemble.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/triggers/envelope.rs");
    let _ = include_str!("../../../../darkmatter/lib/src/markdown/schemas/triggers/prepared.rs");
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
fn the_checked_in_artifact_round_trips_to_uncached_preparation_in_order() {
    let lowers = current_lowers();
    let accepted = mechanism::accept::<ClaudinePayload, _>(
        &shipped::descriptor_over(shipped::sources(), &lowers),
        Some(CHECKED_IN),
        |_, payload| Ok(payload),
    )
    .expect("the checked-in artifact is accepted");
    let uncached = shipped::prepare_payload(&fresh().0).expect("uncached preparation");
    assert_eq!(accepted.payload(), &uncached);
    // `Value` and map equality ignore order; the encoding does not.
    assert_eq!(serde_json::to_string(accepted.payload()).unwrap(), serde_json::to_string(&uncached).unwrap());

    let registry = Registry::load(Some(CHECKED_IN));
    assert_eq!(registry.check_all(), Ok(()));
    let globals: Vec<&str> = registry.globals().globals().map(|global| global.name()).collect();
    assert_eq!(globals, ["seq", "loop", "group"]);
    assert_eq!(
        registry.globals(),
        &GlobalCatalog::from_record(uncached.globals.clone(), claudine::composition::schema::globals::CLAUDINE_GLOBALS)
            .unwrap()
    );
    assert_eq!(registry.bundle().map(|bundle| bundle.bundle()), Some(CLAUDINE_SCHEMA_BUNDLE_NAME));
    let names: Vec<&str> = uncached.bundle.documents.iter().map(|document| document.name.as_str()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "documents keep the bundle's sorted file order");
}

#[test]
fn fresh_generation_is_deterministic() {
    let (lower, darkmatter, claudine) = fresh();
    assert_eq!(claudine.artifact(), FreshClaudine::prepare(lower, darkmatter).unwrap().artifact());
    assert_eq!(claudine.artifact().last(), Some(&b'\n'));
    assert!(!claudine.artifact().ends_with(b"\n\n"));
}

// ── Accepted lookups ────────────────────────────────────────────────────────

/// A cold process discovers Claudine's bundle and reads its host catalog
/// from the accepted artifact: nothing is prepared from text, and no payload
/// merge or compile check runs.
#[cfg(feature = "work-counters")]
#[test]
fn cold_accepted_lookups_prepare_nothing() {
    use claudine::composition::schema::bundle::claudine_schema_bundle;
    use claudine::composition::schema::globals::claudine_globals;
    use darkmatter::work_counters::{Work, snapshot};
    let bare = Bare::new();
    let before = snapshot();
    assert!(shipped::registry().refusal().is_none(), "{:?}", shipped::registry().refusal());
    let effective = bare.effective(claudine_schema_bundle(), "start: []\nprompt: hello");
    assert!(effective.json_schema["properties"].get("start").is_some(), "claudine.yaml applied");
    assert_eq!(claudine_globals().globals().count(), 3);
    let work = snapshot().since(&before);
    assert_eq!(work.get(Work::ShippedRuntimePreparation), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::PayloadMergeCheck), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::PayloadCompileCheck), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::ArtifactAccept), 3, "the library's, Darkmatter's, and Claudine's: {:?}", work.nonzero());
}

/// A bundle named `claudine` with other files is not the bundle the artifact
/// was prepared from: discovery prepares it from its own text.
#[test]
fn a_custom_same_name_bundle_is_prepared_not_served() {
    const FILES: &[(&str, &str)] = &[(
        "claudine.yaml",
        "kind: schema-trigger\nmatch:\n    - prompt: string(required)\n$schema:\n    custom_marker: number\n",
    )];
    assert!(shipped::registry().bundle().is_some(), "the process artifact is accepted");
    let bare = Bare::new();
    #[cfg(feature = "work-counters")]
    let before = darkmatter::work_counters::snapshot();
    let custom = EmbeddedSchemaBundle::new(CLAUDINE_SCHEMA_BUNDLE_NAME, FILES).with_prepared(&shipped::BUNDLE);
    let effective = bare.effective(custom, "prompt: hello");
    let properties = &effective.json_schema["properties"];
    assert!(properties.get("custom_marker").is_some(), "the custom text applied: {properties}");
    assert!(properties.get("start").is_none(), "the shipped claudine.yaml was not served");
    #[cfg(feature = "work-counters")]
    {
        use darkmatter::work_counters::Work;
        let work = darkmatter::work_counters::snapshot().since(&before);
        assert_eq!(work.get(Work::ShippedRuntimePreparation), 1, "{:?}", work.nonzero());
        assert!(work.get(Work::PayloadCompileCheck) >= 1, "{:?}", work.nonzero());
    }
}

/// Generation prepares every owner from text: no artifact is decoded or
/// accepted, so neither lower registry nor Claudine's is consulted.
#[cfg(feature = "work-counters")]
#[test]
fn generation_consults_no_artifact() {
    use darkmatter::work_counters::{Work, snapshot};
    let before = snapshot();
    let _ = fresh();
    let work = snapshot().since(&before);
    assert_eq!(work.get(Work::ArtifactDecode), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::ArtifactAccept), 0, "{:?}", work.nonzero());
    assert_eq!(work.get(Work::ArtifactGeneration), 3, "{:?}", work.nonzero());
}

// ── Fallback: a refused artifact prepares each item from text ───────────────

/// Loads `bytes` over `lower`, checks the refusal (of the whole artifact, or
/// of a section), and checks that the fallback views equal the accepted
/// artifact's.
#[track_caller]
fn assert_falls_back(bytes: Option<&[u8]>, lower: [LowerArtifact; 2], reason: FallbackReason, pointer: &str) {
    let refused = Registry::load_over(bytes, lower);
    let refusal = refused.check_all().expect_err("the artifact or a section is refused");
    assert_eq!(refusal.reason, reason, "{refusal}");
    assert!(refusal.pointer.as_deref().unwrap_or("").starts_with(pointer), "pointer is not under `{pointer}`: {refusal}");
    if refused.refusal().is_some() {
        assert!(refused.bundle().is_none(), "no entry of a refused artifact is served");
    }
    let accepted = Registry::load(Some(CHECKED_IN));
    assert_eq!(refused.globals(), accepted.globals());
}

#[test]
fn missing_bytes_fall_back() {
    assert_falls_back(None, current_lowers(), FallbackReason::MissingArtifact, "");
}

#[test]
fn malformed_json_falls_back() {
    assert_falls_back(Some(&CHECKED_IN[..CHECKED_IN.len() / 2]), current_lowers(), FallbackReason::MalformedArtifact, "");
}

#[test]
fn a_source_mismatch_falls_back() {
    // The artifact describes an edited `globals.yaml`, not the compiled one.
    let mutated: Vec<EmbeddedDocument<'static>> = shipped::sources()
        .iter()
        .map(|document| EmbeddedDocument {
            path: document.path,
            text: if document.path == "claudine/schemas/globals.yaml" {
                Box::leak(format!("{}\n# edited\n", document.text).into_boxed_str())
            } else {
                document.text
            },
        })
        .collect();
    let lowers = current_lowers();
    let bytes = mechanism::generate(&shipped::descriptor_over(&mutated, &lowers), &decoded_checked_in()).unwrap();
    assert_falls_back(Some(&bytes), current_lowers(), FallbackReason::SourceMismatch, "/header/sources/");
}

#[test]
fn another_preparation_revision_falls_back() {
    let bytes = edited(|value| value["header"]["preparation_revision"] = json!(shipped::PREPARATION_REVISION + 1));
    assert_falls_back(Some(&bytes), current_lowers(), FallbackReason::PreparationRevisionMismatch, "/header/preparation_revision");
}

#[test]
fn a_malformed_record_falls_back() {
    let bytes = edited(|value| value["payload"]["globals"]["globals"][0]["name"] = json!(5));
    assert_falls_back(Some(&bytes), current_lowers(), FallbackReason::MalformedArtifact, "/payload/globals/globals/0/name");
    let registry = Registry::load(Some(&bytes));
    assert!(registry.refusal().is_none() && registry.bundle().is_some(), "only the globals section is refused");
}

#[test]
fn a_refused_darkmatter_artifact_falls_back() {
    let [lower, _] = current_lowers();
    let refused = LowerArtifact::Refused { owner: darkmatter::markdown::schemas::shipped::OWNER.into() };
    assert_falls_back(Some(CHECKED_IN), [lower, refused], FallbackReason::DependencyMismatch, "/header/dependencies");
}

#[test]
fn a_changed_darkmatter_artifact_falls_back() {
    use darkmatter::markdown::schemas::shipped as darkmatter_shipped;
    let [lower, _] = current_lowers();
    let changed = LowerArtifact::Current(DependencyFingerprint::of_bytes(
        darkmatter_shipped::OWNER,
        darkmatter_shipped::PREPARATION_REVISION,
        b"another darkmatter artifact",
    ));
    assert_falls_back(Some(CHECKED_IN), [lower, changed], FallbackReason::DependencyMismatch, "/header/dependencies");
}

// ── Payload record validators ───────────────────────────────────────────────

#[track_caller]
fn assert_record_refused(bytes: Vec<u8>, reason: FallbackReason, pointer: &str) {
    let refusal = Registry::load(Some(&bytes)).check_all().expect_err("the artifact or a section is refused");
    assert_eq!(refusal.reason, reason, "{refusal}");
    assert!(refusal.pointer.as_deref().unwrap_or("").starts_with(pointer), "pointer is not under `{pointer}`: {refusal}");
}

#[test]
fn the_re_encoded_checked_in_artifact_is_the_accepted_control() {
    assert_eq!(edited(|_| {}), CHECKED_IN, "re-encoding is byte-identical");
    assert_eq!(Registry::load(Some(CHECKED_IN)).check_all(), Ok(()));
}

#[test]
fn a_record_for_another_bundle_is_refused() {
    let bytes = edited(|value| value["payload"]["bundle"]["bundle"] = json!("other"));
    assert_record_refused(bytes, FallbackReason::UnusableRecord, "/payload/bundle/bundle");
}

#[test]
fn a_bundle_document_outside_the_sources_is_refused() {
    let bytes = edited(|value| value["payload"]["bundle"]["documents"][0]["name"] = json!("missing.yaml"));
    assert_record_refused(bytes, FallbackReason::UnusableRecord, "/payload/bundle/documents/0/name");
}

#[test]
fn a_host_catalog_at_another_path_is_refused() {
    let bytes = edited(|value| value["payload"]["globals"]["identity"] = json!("claudine/schemas/other.yaml"));
    assert_record_refused(bytes, FallbackReason::UnusableRecord, "/payload/globals/identity");
}

#[test]
fn a_global_out_of_order_is_refused() {
    let bytes = edited(|value| value["payload"]["globals"]["globals"].as_array_mut().unwrap().swap(0, 1));
    assert_record_refused(bytes, FallbackReason::UnusableRecord, "/payload/globals/globals/0/name");
}

#[test]
fn a_null_catalog_description_is_none_and_an_absent_one_is_malformed() {
    let null = Registry::load(Some(&edited(|value| value["payload"]["globals"]["description"] = Value::Null)));
    assert_eq!(null.check_all(), Ok(()));
    assert_eq!(null.globals().description(), None);
    let absent = edited(|value| {
        value["payload"]["globals"].as_object_mut().unwrap().shift_remove("description");
    });
    assert_record_refused(absent, FallbackReason::MalformedArtifact, "/payload/globals");
}

// ── The generator command ───────────────────────────────────────────────────

fn compiled_lowers() -> generator::CompiledLowers<'static> {
    generator::CompiledLowers {
        lower: simplified_schema::shipped::registry::ARTIFACT,
        darkmatter: darkmatter::markdown::schemas::shipped::ARTIFACT,
    }
}

fn run_generator(
    arguments: &[&str],
    artifact: &std::path::Path,
    lowers: generator::CompiledLowers<'_>,
    generate: impl FnOnce() -> Result<generator::Generated, String>,
) -> (u8, Vec<u8>, String) {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = generator::run(arguments.iter().map(|argument| argument.to_string()), artifact, lowers, generate, &mut stdout, &mut stderr);
    (code, stdout, String::from_utf8(stderr).unwrap())
}

fn generated() -> Result<generator::Generated, String> {
    let (lower, darkmatter, claudine) = fresh();
    Ok(generator::Generated {
        lower: lower.artifact().to_vec(),
        darkmatter: darkmatter.artifact().to_vec(),
        artifact: claudine.artifact().to_vec(),
    })
}

#[test]
fn the_generator_checks_emits_and_writes() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, CHECKED_IN).unwrap();
    assert_eq!(run_generator(&["--check"], &artifact, compiled_lowers(), generated), (0, Vec::new(), String::new()));
    let stale = generator::CompiledLowers { lower: b"stale lower", darkmatter: b"stale darkmatter" };
    let (code, stdout, _) = run_generator(&[], &artifact, stale, generated);
    assert_eq!((code, stdout.as_slice()), (0, fresh().2.artifact()), "emitting checks no lower artifact");
    std::fs::write(&artifact, b"stale").unwrap();
    assert_eq!(run_generator(&["--write"], &artifact, compiled_lowers(), generated).0, 0);
    assert_eq!(std::fs::read(&artifact).unwrap(), fresh().2.artifact());
    assert_eq!(run_generator(&["--check", "--write"], &artifact, compiled_lowers(), generated).0, 2);
}

#[test]
fn a_stale_lower_artifact_stops_check_and_write_naming_the_root_command() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, b"previous").unwrap();
    let current = compiled_lowers();
    let stale_lowers = [
        ("simplified-schema/lib/generated/shipped.json", generator::CompiledLowers { lower: b"stale", ..current }),
        ("darkmatter/lib/generated/shipped.json", generator::CompiledLowers { darkmatter: b"stale", ..current }),
    ];
    for (stale_path, lowers) in stale_lowers {
        for mode in ["--check", "--write"] {
            let (code, stdout, stderr) = run_generator(&[mode], &artifact, lowers, generated);
            assert_eq!(code, 1, "{mode}");
            assert!(stdout.is_empty());
            assert!(stderr.contains(stale_path) && stderr.contains("just shipped-schemas"), "{stderr}");
        }
    }
    assert_eq!(std::fs::read(&artifact).unwrap(), b"previous");
}

#[test]
fn the_generator_reports_where_a_malformed_artifact_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let supplied = directory.path().join("supplied.json");
    let malformed = edited(|value| value["payload"]["globals"]["identity"] = json!(5));
    std::fs::write(&supplied, &malformed).unwrap();
    let (code, stdout, stderr) = run_generator(
        &["--check", "--artifact", supplied.to_str().unwrap()],
        &directory.path().join("shipped.json"),
        compiled_lowers(),
        generated,
    );
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(
        stderr.contains("/payload/globals/identity") && stderr.contains("line") && stderr.contains("just shipped-schemas"),
        "{stderr}"
    );
    assert_eq!(std::fs::read(&supplied).unwrap(), malformed, "a check writes nothing");
}

#[test]
fn a_failed_generation_leaves_the_artifact_in_place() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, b"previous").unwrap();
    let (code, _, stderr) = run_generator(&["--write"], &artifact, compiled_lowers(), || Err("a payload failed".into()));
    assert_eq!(code, 1);
    assert!(stderr.contains("a payload failed"), "{stderr}");
    let lowers = compiled_lowers();
    let refused = || {
        Ok(generator::Generated {
            lower: lowers.lower.to_vec(),
            darkmatter: lowers.darkmatter.to_vec(),
            artifact: b"{\"not\": \"an artifact\"}\n".to_vec(),
        })
    };
    assert_eq!(run_generator(&["--write"], &artifact, lowers, refused).0, 1);
    assert_eq!(std::fs::read(&artifact).unwrap(), b"previous");
    assert!(!directory.path().join("shipped.json.tmp").exists());
}
