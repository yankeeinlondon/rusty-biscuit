//! Envelope admission: every standalone schema envelope is judged by soft
//! validation against the library's compiled definitions
//! (`simplified-schema/schemas/schema.yaml` and `yaml-schema.yaml`), and
//! those definitions bootstrap without consulting themselves.
//!
//! Each row edits one control fixture once, so every edit is known to be the
//! load-bearing one. The default outcome of an invalid shape is a
//! `SchemaDocument` naming the definition, located at the offending token;
//! the listed exceptions are a number `description` (converts softly) and a
//! `types`-only `kind: schema` (valid).

use serde_json::json;
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::envelope::{
    ENVELOPE_DEFINITIONS, SCHEMA_DEFINITION_PATH, YAML_SCHEMA_DEFINITION_PATH, admit_envelope_text,
};
use simplified_schema::resolve::prepare_embedded_yaml_schema;
use simplified_schema::{
    SchemaError, SimplifiedSchema, StandaloneSchemaEnvelope, claimed_values, parse_standalone_schema_document,
};

const INFORMAL: &str = "description: Reviews\ncategories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n";
const TAGGED: &str = "kind: schema\ndescription: Reviews\ncategories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n";
const YAML_SCHEMA: &str = "kind: yaml-schema\ndescription: Reviews\ncategories: [Core]\n$schema:\n  kind: literal(review; required)\ntypes:\n  Id: string\n";

/// The three controls with the definition each is judged against.
const CONTROLS: [(&str, &str, &str); 3] = [
    ("informal", INFORMAL, SCHEMA_DEFINITION_PATH),
    ("tagged", TAGGED, SCHEMA_DEFINITION_PATH),
    ("yaml-schema", YAML_SCHEMA, YAML_SCHEMA_DEFINITION_PATH),
];

/// Asserts `text` is rejected naming `definition`, at the token `at`, with a
/// message mentioning `pointer`.
#[track_caller]
fn assert_rejected(label: &str, text: &str, definition: &str, pointer: &str, at: &str) {
    match parse_standalone_schema_document(text, "subject.yaml") {
        Err(SchemaError::SchemaDocument { message, span, .. }) => {
            assert!(message.contains(&format!("does not satisfy its definition `{definition}`")), "{label}: {message}");
            assert!(message.contains(pointer), "{label}: expected `{pointer}` in {message}");
            assert_eq!(span.map(|span| &text[span]), Some(at), "{label}: {message}");
        }
        other => panic!("{label}: expected a located envelope error, got {other:?}"),
    }
    let admission = admit_envelope_text(text, "subject.yaml").expect("YAML").expect("recognized");
    assert!(!admission.is_admitted(), "{label}");
    assert_eq!(admission.definition, definition, "{label}");
    assert!(admission.problems.iter().all(|problem| problem.line.is_some()), "{label}: every problem is located");
}

// ── Bootstrap and the shipped definitions ─────────────────────────────────

#[test]
fn the_library_definitions_are_admitted_the_yaml_schema_by_itself() {
    for document in ENVELOPE_DEFINITIONS {
        let admission = admit_envelope_text(document.text, document.path)
            .unwrap_or_else(|error| panic!("{}: {error}", document.path))
            .unwrap_or_else(|| panic!("{} is recognized", document.path));
        assert_eq!(admission.envelope, StandaloneSchemaEnvelope::YamlSchema, "{}", document.path);
        assert_eq!(admission.definition, YAML_SCHEMA_DEFINITION_PATH, "{}", document.path);
        assert!(admission.is_admitted(), "{}: {:#?}", document.path, admission.problems);
    }
}

#[test]
fn darkmatter_s_trigger_definition_is_admitted() {
    let text = include_str!("../../../../darkmatter/schemas/schema-trigger.yaml");
    let admission = admit_envelope_text(text, "darkmatter/schemas/schema-trigger.yaml").unwrap().unwrap();
    assert_eq!(admission.definition, YAML_SCHEMA_DEFINITION_PATH);
    assert!(admission.is_admitted(), "{:#?}", admission.problems);
}

#[test]
fn each_shipped_definition_claims_its_kind() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument { path: SCHEMA_DEFINITION_PATH, text: include_str!("../../../schemas/schema.yaml") },
        EmbeddedDocument { path: YAML_SCHEMA_DEFINITION_PATH, text: include_str!("../../../schemas/yaml-schema.yaml") },
        EmbeddedDocument {
            path: "darkmatter/schemas/schema-trigger.yaml",
            text: include_str!("../../../../darkmatter/schemas/schema-trigger.yaml"),
        },
    ];
    for (path, kind) in [
        (SCHEMA_DEFINITION_PATH, "schema"),
        (YAML_SCHEMA_DEFINITION_PATH, "yaml-schema"),
        ("darkmatter/schemas/schema-trigger.yaml", "schema-trigger"),
    ] {
        let resolved = prepare_embedded_yaml_schema(path, DOCUMENTS, ShippedSources::NONE)
            .unwrap_or_else(|error| panic!("{path}: {error}"));
        assert_eq!(claimed_values(&resolved, "kind").unwrap(), [kind], "{path}");
    }
    // The compiled table is the files on disk.
    assert_eq!(ENVELOPE_DEFINITIONS[0].text, DOCUMENTS[0].text);
    assert_eq!(ENVELOPE_DEFINITIONS[1].text, DOCUMENTS[1].text);
}

#[test]
fn a_yaml_schema_preparation_refuses_other_envelopes() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[EmbeddedDocument { path: "tagged.yaml", text: TAGGED }];
    let error = prepare_embedded_yaml_schema("tagged.yaml", DOCUMENTS, ShippedSources::NONE).unwrap_err();
    assert!(matches!(error, SchemaError::NotASchemaDocument { .. }), "{error}");
}

// ── Controls and field shapes ─────────────────────────────────────────────

#[test]
fn every_control_is_admitted() {
    for (label, text, definition) in CONTROLS {
        let admission = admit_envelope_text(text, "subject.yaml").unwrap().unwrap();
        assert_eq!(admission.definition, definition, "{label}");
        assert!(admission.is_admitted(), "{label}: {:#?}", admission.problems);
        parse_standalone_schema_document(text, "subject.yaml").unwrap_or_else(|error| panic!("{label}: {error}"));
    }
}

#[test]
fn an_unknown_top_level_key_is_rejected_at_the_key() {
    // The informal envelope is not recognized with an unknown key (it stays
    // ordinary YAML), so only the tagged envelopes have this row.
    for (label, text, definition) in &CONTROLS[1..] {
        let edited = text.replacen("description:", "notes: draft\ndescription:", 1);
        assert_rejected(label, &edited, definition, "`/notes`", "notes");
    }
}

#[test]
fn null_metadata_and_payload_carriers_are_rejected() {
    for (label, text, definition) in CONTROLS {
        for (key, original) in [
            ("description", "description: Reviews"),
            ("categories", "categories: [Core]"),
            ("types", "types:\n  Id: string"),
        ] {
            let edited = text.replacen(original, &format!("{key}: null"), 1);
            assert_rejected(&format!("{label} {key}: null"), &edited, definition, &format!("`/{key}`"), "null");
        }
        let schema = text.find("$schema:").unwrap();
        let types = text.find("types:").unwrap();
        let edited = format!("{}$schema: null\n{}", &text[..schema], &text[types..]);
        assert_rejected(&format!("{label} $schema: null"), &edited, definition, "`/$schema`", "null");
    }
}

#[test]
fn a_types_list_is_rejected_as_not_a_mapping() {
    for (label, text, definition) in CONTROLS {
        let edited = text.replacen("types:\n  Id: string", "types: [a, b]", 1);
        assert_rejected(label, &edited, definition, "the declaration must be a mapping, not a list", "[a, b]");
    }
}

#[test]
fn neither_schema_nor_types_is_rejected_and_types_only_is_valid_only_for_kind_schema() {
    let neither = "kind: schema\ndescription: Reviews\n";
    assert_rejected("neither", neither, SCHEMA_DEFINITION_PATH, "is a required property", "kind: schema");
    let types_only = "kind: schema\ndescription: Reviews\ntypes:\n  Id: string\n";
    let document = parse_standalone_schema_document(types_only, "subject.yaml").unwrap().unwrap();
    assert!(!document.exports_schema);
    let yaml_types_only = "kind: yaml-schema\ndescription: Reviews\ntypes:\n  Id: string\n";
    assert_rejected("yaml-schema types-only", yaml_types_only, YAML_SCHEMA_DEFINITION_PATH, "\"$schema\" is a required property", "kind: yaml-schema");
}

#[test]
fn a_number_description_converts_softly_and_payload_carriers_are_never_converted() {
    let text = "kind: schema\ndescription: 123\n$schema:\n  - title: string\n  - count: number\ntypes:\n  Id: string(min(2))\n";
    let document = parse_standalone_schema_document(text, "subject.yaml").unwrap().unwrap();
    assert_eq!(document.description.as_deref(), Some("123"));
    let admission = admit_envelope_text(text, "subject.yaml").unwrap().unwrap();
    assert!(admission.is_admitted(), "{:#?}", admission.problems);
    assert_eq!(admission.value["description"], json!("123"));
    // The root-union sequence and the `types` mapping are the authored values.
    assert_eq!(admission.value["$schema"], json!([{"title": "string"}, {"count": "number"}]));
    assert_eq!(admission.value["types"], json!({"Id": "string(min(2))"}));
    // A scalar carrier (a whole-file reference) is not converted either.
    let reference = admit_envelope_text("description: 5\n$schema: ./other.yaml\n", "s.yaml").unwrap().unwrap();
    assert!(reference.is_admitted(), "{:#?}", reference.problems);
    assert_eq!(reference.value, json!({"description": "5", "$schema": "./other.yaml"}));
}

/// The coercion engine itself never stringifies a `schema` or
/// `type-definition` carrier: a mapping and a sequence pass through soft
/// conversion unchanged while a sibling scalar converts.
#[test]
fn soft_conversion_passes_meta_type_carriers_through_untouched() {
    let schema = SimplifiedSchema::try_from(
        "$schema:\n  note: string\n  types: schema(mapping)\n  arms: schema\n  shape: type-definition\n",
    )
    .unwrap();
    let value = json!({
        "note": 7,
        "types": {"Id": "string"},
        "arms": [{"a": "string"}, "./b.yaml"],
        "shape": {"x": "number"},
    });
    let accepted = schema.validate_soft(&value).expect("accepted");
    assert!(accepted.is_converted());
    assert_eq!(
        accepted.value.as_ref(),
        &json!({"note": "7", "types": {"Id": "string"}, "arms": [{"a": "string"}, "./b.yaml"], "shape": {"x": "number"}})
    );
}

#[test]
fn a_category_entry_that_cannot_be_text_is_rejected_by_the_definition() {
    for (label, text, definition) in CONTROLS {
        let edited = text.replacen("[Core]", "[Core, {a: b}]", 1);
        assert_rejected(label, &edited, definition, "`/categories/1`", "{a: b}");
    }
}

// ── Every direct load admits ──────────────────────────────────────────────

#[test]
fn the_content_factory_rejects_a_malformed_envelope_with_a_located_error() {
    let text = TAGGED.replacen("description: Reviews", "description: null", 1);
    match SimplifiedSchema::try_from(text.as_str()) {
        Err(error) => {
            let message = error.to_string();
            assert!(message.contains(SCHEMA_DEFINITION_PATH), "{message}");
            assert!(message.contains("`/description`"), "{message}");
        }
        Ok(_) => panic!("a malformed envelope must not load"),
    }
}

#[test]
fn an_embedded_import_source_is_admitted_before_its_types_are_read() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument { path: "lib/shared.yaml", text: "kind: schema\nnotes: draft\ntypes:\n  Id: string\n" },
        EmbeddedDocument { path: "lib/consumer.yaml", text: "$schema:\n  id: Id@./shared.yaml\n" },
    ];
    let error = simplified_schema::resolve::prepare_embedded_schema("lib/consumer.yaml", DOCUMENTS, ShippedSources::NONE)
        .expect_err("a malformed import source is refused");
    assert!(error.to_string().contains("'notes' was unexpected"), "{error}");
}

#[cfg(feature = "file")]
#[test]
fn a_file_import_source_is_admitted_before_its_types_are_read() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = biscuit_file::canonicalize_simplified(tmp.path()).unwrap();
    std::fs::write(dir.join("shared.yaml"), "kind: schema\ntypes: [Id]\n").unwrap();
    std::fs::write(dir.join("consumer.yaml"), "$schema:\n  id: Id@./shared.yaml\n").unwrap();
    let context = biscuit_file::FileResolutionContext::new(&dir);
    let error = simplified_schema::resolve::resolve_schema_with_roots(
        &json!("./consumer.yaml"),
        &dir,
        &[],
        &context,
        ShippedSources::NONE,
    )
    .expect_err("a malformed import source is refused");
    let message = error.to_string();
    assert!(message.contains(SCHEMA_DEFINITION_PATH) && message.contains("`/types`"), "{message}");
}

/// Which defect an invalid file reports: the envelope's own problem first,
/// then a definition inside a carrier (located on its token by the parser),
/// then the carrier's shape.
#[test]
fn the_envelope_s_own_problem_outranks_a_definition_which_outranks_the_carrier() {
    let report = |text: &str| match parse_standalone_schema_document(text, "subject.yaml") {
        Err(SchemaError::SchemaDocument { message, span, .. }) => (message, span.map(|span| text[span].to_string())),
        other => panic!("expected a SchemaDocument for {text:?}, got {other:?}"),
    };
    // 1. An unknown key beside a bad definition: the key.
    let (message, at) = report("kind: schema\ntypes:\n  slug: string(min(nope))\nextra: 1\n");
    assert!(message.contains("'extra' was unexpected"), "{message}");
    assert_eq!(at.as_deref(), Some("extra"));
    // A misspelled `types:` is the unknown key, not the payload it leaves missing.
    let (message, at) = report("kind: schema\ntyped:\n  slug: string\n");
    assert!(message.contains("'typed' was unexpected"), "{message}");
    assert_eq!(at.as_deref(), Some("typed"));
    // 2. A bad definition in an otherwise valid envelope: the parser's token.
    let (message, at) = report("kind: schema\ntypes:\n  slug: string(min(nope))\n");
    assert!(message.starts_with("invalid SimplifiedSchema payload"), "{message}");
    assert_eq!(at.as_deref(), Some("nope"));
    let (message, _) = report("kind: schema\n$schema:\n  id: Id\ntypes:\n  Id: string(min(nope))\n");
    assert!(message.starts_with("invalid named types"), "{message}");
    // 3. A carrier of the wrong shape: the definition's problem, never a
    // complaint about the entries of a list that cannot be `types` at all.
    let (message, at) = report("kind: schema\ntypes: [5, true]\n");
    assert!(message.contains("the declaration must be a mapping, not a list"), "{message}");
    assert_eq!(at.as_deref(), Some("[5, true]"));
}

/// A file a host already judged against a definition it selected is read
/// without admission: an unknown key is ignored, metadata comes from the
/// host's accepted copy, and the semantic checks still run. The same text
/// loaded directly is still admitted against the compiled definition.
#[test]
fn a_selected_document_is_read_without_admission_and_keeps_its_semantic_checks() {
    use simplified_schema::parse_selected_standalone_schema_document;

    let text = "kind: schema\nnote: hi\ncategories: [text]\ntypes:\n  Id: string(category(text))\n";
    let accepted = json!({"kind": "schema", "note": "hi", "categories": ["text"], "types": {"Id": "string(category(text))"}});
    let document = parse_selected_standalone_schema_document(text, "subject.yaml", &accepted)
        .expect("read")
        .expect("recognized");
    assert_eq!(document.categories.as_ref().map(|categories| categories[0].name.as_str()), Some("text"));
    assert_rejected("direct load", text, SCHEMA_DEFINITION_PATH, "`/note`", "note");

    let undeclared = text.replace("category(text)", "category(media)");
    let error = parse_selected_standalone_schema_document(&undeclared, "subject.yaml", &accepted).unwrap_err();
    assert!(error.to_string().contains("category `media` is not declared"), "{error}");
    // The categories are the accepted copy's, not the text's.
    let declared = json!({"kind": "schema", "categories": ["text", "media"]});
    assert!(parse_selected_standalone_schema_document(&undeclared, "subject.yaml", &declared).is_ok());
}

#[test]
fn a_selected_yaml_schema_prepares_and_claims_its_kinds() {
    use simplified_schema::parse_selected_standalone_schema_document;
    use simplified_schema::resolve::prepare_standalone_yaml_schema;

    let text = "kind: yaml-schema\nnote: hi\n$schema:\n  kind: literal(review; required)\n";
    let accepted = json!({"kind": "yaml-schema", "note": "hi", "$schema": {"kind": "literal(review; required)"}});
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("review.yaml");
    let document = parse_selected_standalone_schema_document(text, &path, &accepted).unwrap().unwrap();
    let context = biscuit_file::FileResolutionContext::new(dir.path());
    let resolved = prepare_standalone_yaml_schema(document, &[], &context, ShippedSources::NONE).expect("prepared");
    assert_eq!(claimed_values(&resolved, "kind").unwrap(), ["review"]);
}
