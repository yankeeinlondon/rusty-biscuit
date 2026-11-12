//! Shipped artifacts: the checked-in prepared form of compiled-in schema
//! text (`simplified_schema::shipped`).
//!
//! The reader under test is the artifact loader ([`shipped::accept`], and
//! the library's own [`Registry::load`] over it). Its default outcome for
//! every invalid artifact shape is a refusal of the whole artifact with a
//! [`FallbackReason`] and, where known, the JSON pointer and line/column;
//! no entry of it is exposed, the owner prepares the compiled text instead,
//! and validation through the fallback is unchanged. Listed exceptions: an
//! empty `dependencies` or `configuration` list is the valid control (this
//! owner has neither), trailing JSON whitespace is accepted, and a header
//! that only disagrees with current text (a digest, revision, inventory,
//! dependency, or configuration value) is a mismatch reason rather than
//! `MalformedArtifact`.
//!
//! Every row edits one member of a freshly generated, accepted artifact, so
//! the edit is the load-bearing change; nothing here writes a repository
//! file or a production registry.

use std::sync::OnceLock;

use serde_json::{Value, json};
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::envelope::{ENVELOPE_DEFINITIONS, SCHEMA_DEFINITION_PATH};
use simplified_schema::resolve::{prepare_embedded_schema, prepare_embedded_schema_with_mode};
use simplified_schema::shipped::registry::{self, EnvelopePayload, Registry};
use simplified_schema::shipped::{
    self, ArtifactDescriptor, DependencyFingerprint, FallbackReason, FreshLower, LowerArtifact, NamedTypeTableRecord,
    PreparationMode, PreparedNamedTypes, PreparedSourcesDescriptor,
};
use simplified_schema::{DefNamespace, DefinitionTable, PropertyDef, SchemaSourceId};

#[path = "../../examples/shipped_schemas.rs"]
#[allow(dead_code)]
mod generator;

/// The checked-in artifact, as the library embeds it.
const CHECKED_IN: &[u8] = include_bytes!("../../generated/shipped.json");

/// One fresh generation per test process.
fn fresh() -> &'static FreshLower {
    static FRESH: OnceLock<FreshLower> = OnceLock::new();
    FRESH.get_or_init(|| FreshLower::prepare().expect("the compiled envelope definitions generate"))
}

fn fresh_value() -> Value {
    serde_json::from_slice(fresh().artifact()).expect("generation writes JSON")
}

fn encode(value: &Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

/// A valid envelope and an invalid one, admitted through `registry`, as the
/// public result a fallback must reproduce.
fn admission_verdicts(registry: &Registry) -> (bool, Option<String>) {
    let envelopes = registry.envelopes().expect("the definitions prepare");
    let valid = envelopes
        .admit("kind: schema\ndescription: Reviews\n$schema:\n  title: string\n", "valid.yaml")
        .unwrap()
        .expect("an envelope");
    let invalid = envelopes
        .admit("kind: schema\nunknown: 1\n$schema:\n  title: string\n", "invalid.yaml")
        .unwrap()
        .expect("an envelope");
    (valid.is_admitted(), invalid.problems.first().map(|problem| problem.message.clone()))
}

// ── Freshness and round trip ────────────────────────────────────────────────

#[test]
fn shipped_artifact_is_fresh() {
    // The library's only shipped sources are its envelope definitions,
    // compiled in through `ENVELOPE_DEFINITIONS`; this owner has no lower
    // artifact to regenerate.
    let checked_in = include_bytes!("../../generated/shipped.json");
    let fresh = registry::generate().expect("fresh generation");
    if let Err(stale) = shipped::check_fresh(checked_in, &fresh, registry::ARTIFACT_PATH, registry::REGENERATE_COMMAND) {
        panic!("{stale}");
    }
}

#[test]
fn a_mutated_source_fails_freshness_naming_the_regeneration_command() {
    let mutated: Vec<EmbeddedDocument<'static>> = ENVELOPE_DEFINITIONS
        .iter()
        .map(|document| EmbeddedDocument {
            path: document.path,
            text: if document.path == SCHEMA_DEFINITION_PATH {
                Box::leak(format!("{}\n# edited\n", document.text).into_boxed_str())
            } else {
                document.text
            },
        })
        .collect();
    let descriptor = ArtifactDescriptor { sources: &mutated, ..registry::descriptor() };
    let payload: EnvelopePayload = serde_json::from_value(fresh_value()["payload"].clone()).unwrap();
    let regenerated = shipped::generate(&descriptor, &payload).unwrap();
    let stale = shipped::check_fresh(CHECKED_IN, &regenerated, registry::ARTIFACT_PATH, registry::REGENERATE_COMMAND)
        .expect_err("an edited source changes the header");
    assert!(stale.to_string().contains("just shipped-schemas"), "{stale}");
    // The checked-in artifact no longer describes the edited text either.
    let refusal = shipped::accept::<EnvelopePayload, _>(&descriptor, Some(CHECKED_IN), |_, payload| Ok(payload))
        .expect_err("a source mismatch");
    assert_eq!(refusal.reason, FallbackReason::SourceMismatch);
    assert_eq!(refusal.pointer.as_deref(), Some("/header/sources/0/digest"), "{refusal}");
}

#[test]
fn the_checked_in_artifact_round_trips_to_uncached_preparation_in_order() {
    let accepted = shipped::accept::<EnvelopePayload, _>(&registry::descriptor(), Some(CHECKED_IN), |_, payload| Ok(payload))
        .expect("the checked-in artifact is accepted");
    let fresh: EnvelopePayload = serde_json::from_value(fresh_value()["payload"].clone()).unwrap();
    assert_eq!(accepted.payload(), &fresh);
    // `Value` equality ignores object key order; the encoded form does not.
    assert_eq!(serde_json::to_string(accepted.payload()).unwrap(), serde_json::to_string(&fresh).unwrap());
    assert_eq!(accepted.header(), &registry::descriptor().header().unwrap());
}

#[test]
fn the_process_registry_accepts_the_artifact_and_admits_as_the_fallback_does() {
    let loaded = Registry::load(Some(CHECKED_IN));
    assert!(loaded.refusal().is_none(), "{:?}", loaded.refusal());
    let fallback = Registry::load(None);
    assert_eq!(fallback.refusal().map(|refusal| refusal.reason), Some(FallbackReason::MissingArtifact));
    let (valid, invalid) = admission_verdicts(&loaded);
    assert!(valid);
    assert!(invalid.is_some());
    assert_eq!(admission_verdicts(&fallback), (valid, invalid));
}

#[test]
fn an_accepted_artifact_is_what_admission_judges_by() {
    // Control: a payload that differs from the text while its header matches
    // changes the public verdict, so the accepted entries are in use.
    let mut value = fresh_value();
    value["payload"]["definitions"][0]["json_schema"] = json!({"type": "object", "required": ["never"]});
    let loaded = Registry::load(Some(&encode(&value)));
    assert!(loaded.refusal().is_none(), "{:?}", loaded.refusal());
    assert!(!admission_verdicts(&loaded).0, "the recorded schema, not the text, judged the envelope");
}

#[cfg(feature = "work-counters")]
#[test]
fn an_accepted_artifact_prepares_nothing_and_a_refused_one_prepares_once() {
    use simplified_schema::work_counters::{Work, snapshot};
    let before = snapshot();
    let loaded = Registry::load(Some(CHECKED_IN));
    let accepted = snapshot().since(&before);
    assert!(loaded.refusal().is_none());
    assert_eq!(accepted.get(Work::ArtifactAccept), 1);
    assert_eq!(accepted.get(Work::ShippedRuntimePreparation), 0);

    let before = snapshot();
    let _ = Registry::load(Some(b"{}"));
    let refused = snapshot().since(&before);
    assert_eq!(refused.get(Work::ArtifactFallbackMalformed), 1);
    assert_eq!(refused.get(Work::ShippedRuntimePreparation), 1);
    assert_eq!(refused.get(Work::ArtifactAccept), 0);
}

#[test]
fn fresh_generation_ignores_the_artifact_and_the_registry() {
    // Generation takes no artifact bytes at all: it runs whatever the
    // checked-in file holds, and never loads the process registry.
    let fresh = FreshLower::prepare().unwrap();
    assert!(!registry::registry_is_loaded(), "generation initialized the runtime registry");
    assert_eq!(fresh.artifact(), registry::generate().unwrap().as_slice());
    assert_eq!(fresh.artifact().last(), Some(&b'\n'));
    assert!(!fresh.artifact().ends_with(b"\n\n"));
    // Deterministic: the same text generates the same bytes.
    assert_eq!(fresh.artifact(), FreshLower::prepare().unwrap().artifact());
}

// ── Input robustness: one row per edited member ─────────────────────────────

/// Applies `edit` to the payload or header of a fresh artifact, loads it,
/// and checks the refusal, its pointer prefix, that the accepted-entry
/// callback never ran for a pre-record refusal, and that the fallback still
/// admits correctly.
#[track_caller]
fn assert_refused(bytes: Option<Vec<u8>>, reason: FallbackReason, pointer: Option<&str>) {
    let mut validated = false;
    let refusal = shipped::accept::<EnvelopePayload, _>(&registry::descriptor(), bytes.as_deref(), |_, payload| {
        validated = true;
        Ok(payload)
    });
    let loaded = Registry::load(bytes.as_deref());
    let registry_refusal = loaded.refusal().cloned().expect("the artifact is refused");
    assert_eq!(registry_refusal.reason, reason, "{registry_refusal}");
    if let Some(pointer) = pointer {
        let actual = registry_refusal.pointer.as_deref().unwrap_or("");
        assert!(actual.starts_with(pointer), "pointer `{actual}` is not under `{pointer}`: {registry_refusal}");
    }
    if reason != FallbackReason::UnusableRecord || pointer.is_some_and(|pointer| pointer.starts_with("/header")) {
        assert!(refusal.is_err() && !validated, "a refused artifact's records reached validation");
    }
    let (valid, invalid) = admission_verdicts(&loaded);
    assert!(valid && invalid.is_some(), "the fallback admits as ordinary preparation does");
}

fn edited(edit: impl FnOnce(&mut Value)) -> Option<Vec<u8>> {
    let mut value = fresh_value();
    edit(&mut value);
    Some(encode(&value))
}

fn remove(value: &mut Value, pointer: &str) {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    value.pointer_mut(parent).unwrap().as_object_mut().unwrap().shift_remove(key);
}

/// Replaces the member at `pointer`, adding it to its object when absent.
fn set(value: &mut Value, pointer: &str, replacement: Value) {
    if let Some(member) = value.pointer_mut(pointer) {
        *member = replacement;
        return;
    }
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    value.pointer_mut(parent).unwrap().as_object_mut().unwrap().insert(key.to_string(), replacement);
}

macro_rules! refused {
    ($($name:ident: $bytes:expr => $reason:ident at $pointer:expr;)*) => {$(
        #[test]
        fn $name() {
            assert_refused($bytes, FallbackReason::$reason, $pointer);
        }
    )*};
}

const DIGEST: &str = "/header/sources/0/digest";

refused! {
    // Document shapes (one loader).
    missing_bytes_fall_back: None => MissingArtifact at None;
    empty_bytes_are_malformed: Some(Vec::new()) => MalformedArtifact at None;
    whitespace_only_is_malformed: Some(b"  \n".to_vec()) => MalformedArtifact at None;
    invalid_utf8_is_malformed: Some(vec![b'{', 0xff, b'}']) => MalformedArtifact at None;
    truncated_json_is_malformed: Some(fresh().artifact()[..fresh().artifact().len() / 2].to_vec()) => MalformedArtifact at None;
    trailing_content_is_malformed: Some([fresh().artifact(), b"{}"].concat()) => MalformedArtifact at None;
    a_json5_comment_is_malformed: Some([b"// generated\n".as_slice(), fresh().artifact()].concat()) => MalformedArtifact at None;
    a_trailing_comma_is_malformed: Some(String::from_utf8(fresh().artifact().to_vec()).unwrap().replacen("\"payload\"", "\"x\": 1,,\"payload\"", 1).into_bytes()) => MalformedArtifact at None;
    a_repeated_top_level_key_is_malformed: Some(String::from_utf8(fresh().artifact().to_vec()).unwrap().replacen("{\n\"header\"", "{\n\"payload\": null,\n\"header\"", 1).into_bytes()) => MalformedArtifact at Some("/payload");
    a_repeated_key_inside_a_payload_schema_is_malformed: Some(String::from_utf8(fresh().artifact().to_vec()).unwrap().replacen("\"json_schema\":{", "\"json_schema\":{\"$schema\":\"x\",", 1).into_bytes()) => MalformedArtifact at Some("/payload/definitions/0/json_schema/$schema");
    an_unknown_top_level_member_is_malformed: edited(|value| set(value, "/extra", json!(1))) => MalformedArtifact at None;
    a_missing_payload_is_malformed: edited(|value| remove(value, "/payload")) => MalformedArtifact at None;
    a_null_payload_is_malformed: edited(|value| set(value, "/payload", Value::Null)) => MalformedArtifact at Some("/payload");

    // `wire_version`: a required u32 scalar.
    absent_wire_version: edited(|value| remove(value, "/header/wire_version")) => MalformedArtifact at Some("/header");
    null_wire_version: edited(|value| set(value, "/header/wire_version", Value::Null)) => MalformedArtifact at Some("/header/wire_version");
    string_wire_version: edited(|value| set(value, "/header/wire_version", json!("1"))) => MalformedArtifact at Some("/header/wire_version");
    negative_wire_version: edited(|value| set(value, "/header/wire_version", json!(-1))) => MalformedArtifact at Some("/header/wire_version");
    another_wire_version: edited(|value| set(value, "/header/wire_version", json!(2))) => UnsupportedWireVersion at Some("/header/wire_version");

    // `owner`: a required non-empty string naming this owner.
    absent_owner: edited(|value| remove(value, "/header/owner")) => MalformedArtifact at Some("/header");
    null_owner: edited(|value| set(value, "/header/owner", Value::Null)) => MalformedArtifact at Some("/header/owner");
    numeric_owner: edited(|value| set(value, "/header/owner", json!(5))) => MalformedArtifact at Some("/header/owner");
    empty_owner: edited(|value| set(value, "/header/owner", json!(""))) => MalformedArtifact at Some("/header/owner");
    another_owner: edited(|value| set(value, "/header/owner", json!("darkmatter"))) => UnusableRecord at Some("/header/owner");

    // `preparation_revision`: a required u32 of at least 1.
    absent_revision: edited(|value| remove(value, "/header/preparation_revision")) => MalformedArtifact at Some("/header");
    null_revision: edited(|value| set(value, "/header/preparation_revision", Value::Null)) => MalformedArtifact at Some("/header/preparation_revision");
    string_revision: edited(|value| set(value, "/header/preparation_revision", json!("1"))) => MalformedArtifact at Some("/header/preparation_revision");
    zero_revision: edited(|value| set(value, "/header/preparation_revision", json!(0))) => MalformedArtifact at Some("/header/preparation_revision");
    another_revision: edited(|value| set(value, "/header/preparation_revision", json!(2))) => PreparationRevisionMismatch at Some("/header/preparation_revision");

    // `sources`: the complete, sorted inventory.
    absent_sources: edited(|value| remove(value, "/header/sources")) => MalformedArtifact at Some("/header");
    null_sources: edited(|value| set(value, "/header/sources", Value::Null)) => MalformedArtifact at Some("/header/sources");
    object_sources: edited(|value| set(value, "/header/sources", json!({}))) => MalformedArtifact at Some("/header/sources");
    empty_sources: edited(|value| set(value, "/header/sources", json!([]))) => SourceMismatch at Some("/header/sources");
    one_invalid_source: edited(|value| value["header"]["sources"].as_array_mut().unwrap().push(json!(5))) => MalformedArtifact at Some("/header/sources/2");
    every_source_invalid: edited(|value| set(value, "/header/sources", json!([5, 6]))) => MalformedArtifact at Some("/header/sources/0");
    a_source_with_an_unknown_member: edited(|value| set(value, "/header/sources/0/extra", json!(1))) => MalformedArtifact at Some("/header/sources/0");
    a_repeated_source: edited(|value| { let first = value["header"]["sources"][0].clone(); value["header"]["sources"].as_array_mut().unwrap().insert(0, first); }) => MalformedArtifact at Some("/header/sources/1");
    unsorted_sources: edited(|value| value["header"]["sources"].as_array_mut().unwrap().reverse()) => MalformedArtifact at Some("/header/sources/1");
    an_empty_source_identity: edited(|value| set(value, "/header/sources/0/identity", json!(""))) => MalformedArtifact at Some("/header/sources/0/identity");
    a_renamed_source: edited(|value| set(value, "/header/sources/0/identity", json!("simplified-schema/schemas/other.yaml"))) => SourceMismatch at Some("/header/sources/0/identity");
    an_extra_source: edited(|value| value["header"]["sources"].as_array_mut().unwrap().push(json!({"identity": "zz.yaml", "digest": "0000000000000000"}))) => SourceMismatch at Some("/header/sources");

    // The digest validator, shared by sources and dependencies.
    absent_digest: edited(|value| remove(value, DIGEST)) => MalformedArtifact at Some("/header/sources/0");
    null_digest: edited(|value| set(value, DIGEST, Value::Null)) => MalformedArtifact at Some(DIGEST);
    numeric_digest: edited(|value| set(value, DIGEST, json!(5))) => MalformedArtifact at Some(DIGEST);
    empty_digest: edited(|value| set(value, DIGEST, json!(""))) => MalformedArtifact at Some(DIGEST);
    short_digest: edited(|value| set(value, DIGEST, json!("0123456789abcde"))) => MalformedArtifact at Some(DIGEST);
    long_digest: edited(|value| set(value, DIGEST, json!("0123456789abcdef0"))) => MalformedArtifact at Some(DIGEST);
    uppercase_digest: edited(|value| set(value, DIGEST, json!("0123456789ABCDEF"))) => MalformedArtifact at Some(DIGEST);
    non_hex_digest: edited(|value| set(value, DIGEST, json!("0123456789abcdeg"))) => MalformedArtifact at Some(DIGEST);
    a_changed_digest: edited(|value| set(value, DIGEST, json!("0000000000000000"))) => SourceMismatch at Some(DIGEST);

    // `dependencies`: this owner has none; `[]` is the control.
    absent_dependencies: edited(|value| remove(value, "/header/dependencies")) => MalformedArtifact at Some("/header");
    null_dependencies: edited(|value| set(value, "/header/dependencies", Value::Null)) => MalformedArtifact at Some("/header/dependencies");
    string_dependencies: edited(|value| set(value, "/header/dependencies", json!("none"))) => MalformedArtifact at Some("/header/dependencies");
    an_invalid_dependency: edited(|value| set(value, "/header/dependencies", json!([5]))) => MalformedArtifact at Some("/header/dependencies/0");
    a_dependency_digest_wired_to_the_validator: edited(|value| set(value, "/header/dependencies", json!([{"owner": "lower", "preparation_revision": 1, "digest": "XYZ"}]))) => MalformedArtifact at Some("/header/dependencies/0/digest");
    a_zero_revision_dependency: edited(|value| set(value, "/header/dependencies", json!([{"owner": "lower", "preparation_revision": 0, "digest": "0000000000000000"}]))) => MalformedArtifact at Some("/header/dependencies/0/preparation_revision");
    an_unexpected_dependency: edited(|value| set(value, "/header/dependencies", json!([{"owner": "lower", "preparation_revision": 1, "digest": "0000000000000000"}]))) => DependencyMismatch at Some("/header/dependencies");

    // `configuration`: this owner has none; `[]` is the control.
    absent_configuration: edited(|value| remove(value, "/header/configuration")) => MalformedArtifact at Some("/header");
    null_configuration: edited(|value| set(value, "/header/configuration", Value::Null)) => MalformedArtifact at Some("/header/configuration");
    object_configuration: edited(|value| set(value, "/header/configuration", json!({"a": "b"}))) => MalformedArtifact at Some("/header/configuration");
    a_malformed_configuration_pair: edited(|value| set(value, "/header/configuration", json!([["a"]]))) => MalformedArtifact at Some("/header/configuration/0");
    every_configuration_entry_invalid: edited(|value| set(value, "/header/configuration", json!([5, 6]))) => MalformedArtifact at Some("/header/configuration/0");
    an_unexpected_configuration: edited(|value| set(value, "/header/configuration", json!([["feature", "x"]]))) => ConfigurationMismatch at Some("/header/configuration");

    // Payload records.
    absent_definitions: edited(|value| remove(value, "/payload/definitions")) => MalformedArtifact at Some("/payload");
    null_definitions: edited(|value| set(value, "/payload/definitions", Value::Null)) => MalformedArtifact at Some("/payload/definitions");
    empty_definitions: edited(|value| set(value, "/payload/definitions", json!([]))) => UnusableRecord at Some("/payload/definitions");
    one_invalid_definition: edited(|value| set(value, "/payload/definitions/1", json!(5))) => MalformedArtifact at Some("/payload/definitions/1");
    every_definition_invalid: edited(|value| set(value, "/payload/definitions", json!([5, 6]))) => MalformedArtifact at Some("/payload/definitions/0");
    a_definition_with_an_unknown_member: edited(|value| set(value, "/payload/definitions/0/extra", json!(1))) => MalformedArtifact at Some("/payload/definitions/0");
    a_definition_without_its_schema: edited(|value| remove(value, "/payload/definitions/0/json_schema")) => MalformedArtifact at Some("/payload/definitions/0");
    a_misidentified_definition: edited(|value| set(value, "/payload/definitions/0/identity", json!("other.yaml"))) => UnusableRecord at Some("/payload/definitions/0/identity");
    swapped_definitions: edited(|value| value["payload"]["definitions"].as_array_mut().unwrap().reverse()) => UnusableRecord at Some("/payload/definitions/0/identity");
    a_null_definition_schema: edited(|value| set(value, "/payload/definitions/0/json_schema", Value::Null)) => UnusableRecord at Some("/payload/definitions/0/json_schema");
}

#[test]
fn trailing_whitespace_after_the_document_is_accepted() {
    let bytes = [fresh().artifact(), b"\n  \n".as_slice()].concat();
    assert!(Registry::load(Some(&bytes)).refusal().is_none());
}

#[test]
fn a_syntax_refusal_carries_the_parser_position() {
    let refusal = registry::check_artifact(b"{\n  \"header\": {,\n}").expect_err("malformed");
    assert_eq!(refusal.reason, FallbackReason::MalformedArtifact);
    assert_eq!((refusal.line, refusal.column), (Some(2), Some(14)), "{refusal}");
}

#[test]
fn a_payload_record_refusal_is_located_in_the_whole_artifact() {
    let bytes = edited(|value| set(value, "/payload/definitions/1", json!(5))).unwrap();
    let refusal = registry::check_artifact(&bytes).expect_err("malformed");
    let text = String::from_utf8(bytes).unwrap();
    let line = refusal.line.expect("a line");
    assert!(
        text.lines().nth(line - 1).is_some_and(|authored| authored.contains('5')),
        "line {line} is not the edited record: {refusal}"
    );
}

// ── Lower artifacts ─────────────────────────────────────────────────────────

/// A host artifact prepared over this library's freshly generated artifact.
fn host_descriptor(dependencies: &[LowerArtifact]) -> ArtifactDescriptor<'_> {
    const HOST_SOURCES: &[EmbeddedDocument<'static>] =
        &[EmbeddedDocument { path: "host/schemas/host.yaml", text: "kind: schema\ntypes:\n  Name: string\n" }];
    ArtifactDescriptor {
        owner: "host",
        preparation_revision: 1,
        sources: HOST_SOURCES,
        dependencies,
        configuration: &[("feature", "on")],
    }
}

#[test]
fn a_host_artifact_is_accepted_over_its_current_lower_artifact() {
    let generated = shipped::generate(&host_descriptor(&[fresh().as_dependency()]), &json!({"value": 1})).unwrap();
    let current = [Registry::load(Some(CHECKED_IN)).as_dependency()];
    let accepted = shipped::accept::<Value, _>(&host_descriptor(&current), Some(&generated), |_, payload| Ok(payload))
        .expect("the host artifact matches its current lower artifact");
    assert_eq!(accepted.payload(), &json!({"value": 1}));
}

#[test]
fn a_refused_lower_artifact_refuses_the_host_even_when_its_bytes_match_the_header() {
    // The lower artifact is stale (an older revision), and the host was
    // generated over exactly those stale bytes, so the header digests agree.
    let mut stale = fresh_value();
    set(&mut stale, "/header/preparation_revision", json!(registry::PREPARATION_REVISION + 1));
    let stale = encode(&stale);
    let over_stale = DependencyFingerprint::of_bytes(registry::OWNER, registry::PREPARATION_REVISION, &stale);
    let generated =
        shipped::generate(&host_descriptor(&[LowerArtifact::Current(over_stale)]), &json!({"value": 1})).unwrap();
    let lower = Registry::load(Some(&stale));
    assert_eq!(lower.refusal().map(|refusal| refusal.reason), Some(FallbackReason::PreparationRevisionMismatch));
    let refusal =
        shipped::accept::<Value, _>(&host_descriptor(&[lower.as_dependency()]), Some(&generated), |_, payload| Ok(payload))
            .expect_err("a refused lower artifact refuses its dependents");
    assert_eq!(refusal.reason, FallbackReason::DependencyMismatch);
}

#[test]
fn a_changed_lower_artifact_refuses_the_host() {
    let generated = shipped::generate(&host_descriptor(&[fresh().as_dependency()]), &json!(null)).unwrap();
    let changed = DependencyFingerprint::of_bytes(registry::OWNER, registry::PREPARATION_REVISION, b"other bytes");
    let refusal = shipped::accept::<Value, _>(
        &host_descriptor(&[LowerArtifact::Current(changed)]),
        Some(&generated),
        |_, payload| Ok(payload),
    )
    .expect_err("a different lower artifact");
    assert_eq!(refusal.reason, FallbackReason::DependencyMismatch);
    let current = [fresh().as_dependency()];
    let reconfigured = ArtifactDescriptor { configuration: &[("feature", "off")], ..host_descriptor(&current) };
    let refusal = shipped::accept::<Value, _>(&reconfigured, Some(&generated), |_, payload| Ok(payload))
        .expect_err("a different configuration");
    assert_eq!(refusal.reason, FallbackReason::ConfigurationMismatch);
}

#[test]
fn a_descriptor_that_cannot_generate_is_refused_before_encoding() {
    for (descriptor, expected) in [
        (ArtifactDescriptor { preparation_revision: 0, ..registry::descriptor() }, "at least 1"),
        (ArtifactDescriptor { owner: "", ..registry::descriptor() }, "non-empty"),
        (ArtifactDescriptor { configuration: &[("a", "1"), ("a", "2")], ..registry::descriptor() }, "listed twice"),
    ] {
        let error = shipped::generate(&descriptor, &json!(null)).expect_err(expected);
        assert!(error.contains(expected), "{error}");
    }
    let refused = [LowerArtifact::Refused { owner: "lower".into() }];
    let error = shipped::generate(&host_descriptor(&refused), &json!(null)).expect_err("refused lower");
    assert!(error.contains("not current"), "{error}");
}

// ── Prepared named-type tables for a reserved namespace ─────────────────────

const HOST_PARTIAL: &str = "host/partials/types.yaml";
const HOST_DOCUMENTS: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument { path: HOST_PARTIAL, text: "kind: schema\ntypes:\n  Thing: string(min(2))\n" },
    EmbeddedDocument { path: "host/main.yaml", text: "kind: schema\n$schema:\n  name: Thing@host:types\n" },
];
/// The same paths with another text: a caller's own table, not the host's.
const CUSTOM_DOCUMENTS: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument { path: HOST_PARTIAL, text: "kind: schema\ntypes:\n  Thing: number\n" },
    EmbeddedDocument { path: "host/main.yaml", text: "kind: schema\n$schema:\n  name: Thing@host:types\n" },
];

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct HostPayload {
    inventory: Vec<String>,
    tables: Vec<NamedTypeTableRecord>,
}

fn host_tables_descriptor() -> ArtifactDescriptor<'static> {
    ArtifactDescriptor { owner: "host", preparation_revision: 1, sources: HOST_DOCUMENTS, dependencies: &[], configuration: &[] }
}

/// The host's accepted tables, built from `payload` (which may be edited).
fn accept_host_tables(payload: &HostPayload) -> Result<PreparedNamedTypes, shipped::Refusal> {
    let bytes = shipped::generate(&host_tables_descriptor(), payload).unwrap();
    shipped::accept::<HostPayload, _>(&host_tables_descriptor(), Some(&bytes), |acceptance, payload| {
        PreparedNamedTypes::from_records(acceptance, "", &payload.inventory, payload.tables)
    })
    .map(|accepted| {
        let mut accepted = Some(accepted);
        let mut tables = None;
        let _ = accepted.take().unwrap().map(|prepared| tables = Some(prepared));
        tables.unwrap()
    })
}

fn fresh_host_payload() -> HostPayload {
    let fresh = fresh();
    HostPayload {
        inventory: HOST_DOCUMENTS.iter().map(|document| document.path.to_string()).collect(),
        tables: vec![NamedTypeTableRecord::prepare(&HOST_DOCUMENTS[0], PreparationMode::Uncached(fresh)).unwrap()],
    }
}

/// The arm of an optional property's lowered `anyOf` that is not `null`.
fn non_null_arm(property: &Value) -> Value {
    property["anyOf"]
        .as_array()
        .and_then(|arms| arms.iter().find(|arm| arm["type"] != json!("null")))
        .cloned()
        .unwrap_or_else(|| panic!("an optional property: {property}"))
}

/// The resolved `name` property's lowered type.
fn resolved_name_type(documents: &'static [EmbeddedDocument<'static>], shipped: ShippedSources) -> Value {
    let resolved = prepare_embedded_schema("host/main.yaml", documents, shipped).expect("the importer resolves");
    non_null_arm(&resolved.json_schema["properties"]["name"])
}

static ALTERED: OnceLock<PreparedNamedTypes> = OnceLock::new();
static ALTERED_DESCRIPTOR: PreparedSourcesDescriptor =
    PreparedSourcesDescriptor { owner: "host", provider: || ALTERED.get() };

#[test]
fn an_attached_prepared_table_serves_the_reserved_namespace() {
    // Control: the recorded table differs from the text, so a resolution
    // that reads it is visibly using the artifact.
    let mut payload = fresh_host_payload();
    payload.tables[0].properties = CUSTOM_TABLE.clone();
    ALTERED.set(accept_host_tables(&payload).expect("accepted")).ok();
    let with_prepared =
        ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS).with_prepared(&ALTERED_DESCRIPTOR);
    assert_eq!(resolved_name_type(HOST_DOCUMENTS, with_prepared)["type"], json!("number"));
    let plain = ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS);
    assert_eq!(resolved_name_type(HOST_DOCUMENTS, plain)["type"], json!("string"));
}

static CUSTOM_TABLE: std::sync::LazyLock<indexmap::IndexMap<String, PropertyDef>> = std::sync::LazyLock::new(|| {
    NamedTypeTableRecord::prepare(&CUSTOM_DOCUMENTS[0], PreparationMode::Runtime).unwrap().properties
});

static FAITHFUL: OnceLock<PreparedNamedTypes> = OnceLock::new();
static FAITHFUL_DESCRIPTOR: PreparedSourcesDescriptor =
    PreparedSourcesDescriptor { owner: "host", provider: || FAITHFUL.get() };

#[test]
fn a_caller_table_under_the_host_s_paths_is_parsed_not_served() {
    FAITHFUL.set(accept_host_tables(&fresh_host_payload()).expect("accepted")).ok();
    let custom = ShippedSources::new("host:", "host/partials", CUSTOM_DOCUMENTS).with_prepared(&FAITHFUL_DESCRIPTOR);
    assert_eq!(resolved_name_type(CUSTOM_DOCUMENTS, custom)["type"], json!("number"));
    let host = ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS).with_prepared(&FAITHFUL_DESCRIPTOR);
    assert_eq!(resolved_name_type(HOST_DOCUMENTS, host)["type"], json!("string"));
}

#[cfg(feature = "work-counters")]
#[test]
fn a_served_table_parses_nothing_and_a_mismatched_one_parses_once() {
    use simplified_schema::work_counters::{Work, snapshot};
    FAITHFUL.set(accept_host_tables(&fresh_host_payload()).expect("accepted")).ok();
    let host = ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS).with_prepared(&FAITHFUL_DESCRIPTOR);
    let before = snapshot();
    resolved_name_type(HOST_DOCUMENTS, host);
    assert_eq!(snapshot().since(&before).get(Work::ShippedRuntimePreparation), 0);
    let custom = ShippedSources::new("host:", "host/partials", CUSTOM_DOCUMENTS).with_prepared(&FAITHFUL_DESCRIPTOR);
    let before = snapshot();
    resolved_name_type(CUSTOM_DOCUMENTS, custom);
    assert_eq!(snapshot().since(&before).get(Work::ShippedRuntimePreparation), 1);
}

#[test]
fn uncached_resolution_parses_even_with_a_prepared_table_attached() {
    let mut payload = fresh_host_payload();
    payload.tables[0].properties = CUSTOM_TABLE.clone();
    ALTERED.set(accept_host_tables(&payload).expect("accepted")).ok();
    let with_prepared =
        ShippedSources::new("host:", "host/partials", HOST_DOCUMENTS).with_prepared(&ALTERED_DESCRIPTOR);
    let (resolved, reads) = prepare_embedded_schema_with_mode(
        "host/main.yaml",
        HOST_DOCUMENTS,
        "",
        with_prepared,
        PreparationMode::Uncached(fresh()),
    )
    .unwrap();
    assert_eq!(non_null_arm(&resolved.json_schema["properties"]["name"])["type"], json!("string"));
    assert!(reads.iter().any(|read| read.to_str() == Some(HOST_PARTIAL)), "{reads:?}");
}

#[test]
fn table_records_refuse_a_document_outside_the_inventory_or_twice() {
    let mut outside = fresh_host_payload();
    outside.tables[0].identity = "host/partials/other.yaml".into();
    let refusal = accept_host_tables(&outside).expect_err("outside the inventory");
    assert_eq!((refusal.reason, refusal.pointer.as_deref()), (FallbackReason::UnusableRecord, Some("/payload/tables/0")));
    let mut twice = fresh_host_payload();
    twice.tables.push(twice.tables[0].clone());
    let refusal = accept_host_tables(&twice).expect_err("two tables for one document");
    assert_eq!(refusal.pointer.as_deref(), Some("/payload/tables/1"));
    let mut unknown = fresh_host_payload();
    unknown.inventory.push("host/unknown.yaml".into());
    let refusal = accept_host_tables(&unknown).expect_err("an inventory entry the header lacks");
    assert_eq!(refusal.pointer.as_deref(), Some("/payload/inventory/2"));
}

// ── Language records ────────────────────────────────────────────────────────

const RICH_TYPES: &str = "kind: schema\ntypes:\n  Zeta: string(min(2); pattern(^z); suggest(zed, zulu)) -> last letter\n  Alpha: number(min(0); max(10.5); default(1.0))\n  Mode: enum(fast, slow; default(fast))\n  Pair: tuple([string, number?])\n  Either: string | number[]\n  Shape:\n    id: alphanumeric(allow-dashes; required)\n    <starting::x->: boolean\n  Exact: literal(\"1\")\n  Native: literal(1)\n  Call: \"function(parameters(x: unknown | null); returns(boolean); narrows(x: unknown[]; true-iff); example(is_array([1]) => true))\"\n";

#[test]
fn a_named_type_table_round_trips_in_authored_order() {
    let document = EmbeddedDocument { path: "host/rich.yaml", text: RICH_TYPES };
    let record = NamedTypeTableRecord::prepare(&document, PreparationMode::Runtime).expect("the rich table parses");
    let names: Vec<&str> = record.properties.keys().map(String::as_str).collect();
    assert_eq!(names, ["Zeta", "Alpha", "Mode", "Pair", "Either", "Shape", "Exact", "Native", "Call"]);
    let encoded = serde_json::to_string(&record).unwrap();
    let decoded: NamedTypeTableRecord = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, record);
    assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded, "re-encoding is byte-identical");
    // A quoted scalar stays a string and a native one a number.
    assert!(encoded.contains("\"LiteralValue\":\"1\""), "{encoded}");
    assert!(encoded.contains("\"LiteralValue\":1}"), "{encoded}");
}

#[test]
fn an_absent_optional_member_is_an_error_and_null_is_none() {
    let atom = json!({"ty": {"Primitive": "String"}, "is_array": false, "constraints": [], "array_constraints": []});
    let absent = serde_json::from_value::<PropertyDef>(json!({ "Single": atom.clone() }));
    assert!(absent.is_err_and(|error| error.to_string().contains("description")));
    let mut with_null = atom;
    with_null["description"] = Value::Null;
    let decoded: PropertyDef = serde_json::from_value(json!({ "Single": with_null })).unwrap();
    let PropertyDef::Single(atom) = decoded else { panic!("a single atom") };
    assert_eq!(atom.description, None);
}

#[test]
fn a_definition_table_round_trips_as_ordered_pairs() {
    const RECURSIVE: &[EmbeddedDocument<'static>] = &[EmbeddedDocument {
        path: "host/tree.yaml",
        text: "kind: schema\n$schema:\n  root: Node\ntypes:\n  Node:\n    name: string\n    children: Node[]\n",
    }];
    let resolved = prepare_embedded_schema("host/tree.yaml", RECURSIVE, ShippedSources::NONE).unwrap();
    assert!(!resolved.definitions.is_empty(), "the recursive type is stored");
    let encoded = serde_json::to_value(resolved.definitions.as_ref()).unwrap();
    assert!(encoded.is_array() && encoded[0].is_array(), "{encoded}");
    let decoded: DefinitionTable = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(&decoded, resolved.definitions.as_ref());
    let (key, definition) = decoded.iter().next().unwrap();
    assert_eq!(decoded.key_for_json(&definition.json_key), Some(key));
    // A repeated entry is refused rather than last-wins.
    let doubled = json!([encoded[0].clone(), encoded[0].clone()]);
    assert!(serde_json::from_value::<DefinitionTable>(doubled).is_err());
}

#[test]
fn a_file_backed_identity_is_never_shipped() {
    let file = DefNamespace::File(std::path::PathBuf::from("/home/user/types.yaml"));
    assert!(serde_json::to_value(&file).is_err());
    assert!(serde_json::from_value::<DefNamespace>(json!({"File": "/home/user/types.yaml"})).is_err());
    assert!(serde_json::from_value::<DefNamespace>(json!({"TriggerPayload": "/repo/t.yaml"})).is_err());
    assert!(serde_json::to_value(SchemaSourceId::File("/home/user/types.yaml".into())).is_err());
    assert!(serde_json::from_value::<SchemaSourceId>(json!({"File": "/x"})).is_err());
    assert_eq!(
        serde_json::from_value::<DefNamespace>(json!({"Shipped": "darkmatter/schemas/partials/fm.yaml"})).unwrap(),
        DefNamespace::Shipped("darkmatter/schemas/partials/fm.yaml".into())
    );
}

// ── The generator command ───────────────────────────────────────────────────

fn run_generator(arguments: &[&str], artifact: &std::path::Path, generate: impl FnOnce() -> Result<Vec<u8>, String>) -> (u8, Vec<u8>, String) {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = generator::run(arguments.iter().map(|argument| argument.to_string()), artifact, generate, &mut stdout, &mut stderr);
    (code, stdout, String::from_utf8(stderr).unwrap())
}

#[test]
fn the_generator_checks_emits_and_writes() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, CHECKED_IN).unwrap();
    let generate = || Ok(fresh().artifact().to_vec());
    assert_eq!(run_generator(&["--check"], &artifact, generate), (0, Vec::new(), String::new()));
    let (code, stdout, _) = run_generator(&[], &artifact, generate);
    assert_eq!((code, stdout.as_slice()), (0, fresh().artifact()));
    std::fs::write(&artifact, b"stale").unwrap();
    assert_eq!(run_generator(&["--write"], &artifact, generate).0, 0);
    assert_eq!(std::fs::read(&artifact).unwrap(), fresh().artifact());
    assert_eq!(run_generator(&["--check", "--write"], &artifact, generate).0, 2);
    assert_eq!(run_generator(&["--write", "--artifact", "x"], &artifact, generate).0, 2);
}

#[test]
fn the_generator_reports_where_a_malformed_artifact_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    let malformed = edited(|value| set(value, DIGEST, json!("XYZ"))).unwrap();
    let supplied = directory.path().join("supplied.json");
    std::fs::write(&supplied, &malformed).unwrap();
    let (code, stdout, stderr) =
        run_generator(&["--check", "--artifact", supplied.to_str().unwrap()], &artifact, || Ok(fresh().artifact().to_vec()));
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains(DIGEST) && stderr.contains("line") && stderr.contains("just shipped-schemas"), "{stderr}");
    assert_eq!(std::fs::read(&supplied).unwrap(), malformed, "a check writes nothing");
}

#[test]
fn a_failed_generation_leaves_the_artifact_in_place() {
    let directory = tempfile::tempdir().unwrap();
    let artifact = directory.path().join("shipped.json");
    std::fs::write(&artifact, b"previous").unwrap();
    let (code, _, stderr) = run_generator(&["--write"], &artifact, || Err("a definition failed".into()));
    assert_eq!(code, 1);
    assert!(stderr.contains("a definition failed"), "{stderr}");
    let (code, _, _) = run_generator(&["--write"], &artifact, || Ok(b"{\"not\": \"an artifact\"}\n".to_vec()));
    assert_eq!(code, 1);
    assert_eq!(std::fs::read(&artifact).unwrap(), b"previous");
    assert!(!directory.path().join("shipped.json.tmp").exists());
}
