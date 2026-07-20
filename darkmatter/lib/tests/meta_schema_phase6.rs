//! Phase 6 public parser-product coverage for standalone schema authoring.

use std::path::Path;

use darkmatter::markdown::schemas::{
    SchemaDeclaration, SchemaError, SchemaReferenceKind, SchemaSourcePath, SchemaSpanKind,
    SimplifiedSchema, StandaloneSchemaEnvelope, parse_standalone_schema_document,
};

#[test]
fn shipped_base_schema_exposes_structural_spans_across_anchors_and_aliases() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas/darkmatter.yaml");
    let source = std::fs::read_to_string(&path).expect("read shipped base schema");
    let document = parse_standalone_schema_document(&source, &path)
        .expect("classify shipped base schema")
        .expect("pure standalone schema");
    assert_eq!(document.envelope, StandaloneSchemaEnvelope::Pure);

    let page = SchemaSourcePath::root().property("style").property("page");
    let anchored_union = page.property("left-margin");
    let key = document
        .source_map
        .spans(&anchored_union, SchemaSpanKind::MappingKey)
        .first()
        .expect("anchored key span");
    assert_eq!(&source[key.clone()], "left-margin");
    let first_type = document
        .source_map
        .spans(&anchored_union.union_arm(0), SchemaSpanKind::TypeKeyword)
        .first()
        .expect("type within anchored union");
    assert_eq!(&source[first_type.clone()], "string");

    let alias = page.property("right-margin");
    let alias_definition = document
        .source_map
        .spans(&alias, SchemaSpanKind::Definition)
        .first()
        .expect("alias definition span");
    assert_eq!(&source[alias_definition.clone()], "*style-length");
    assert!(
        document
            .source_map
            .spans(&alias, SchemaSpanKind::TypeKeyword)
            .is_empty(),
        "an alias span must not invent type tokens authored at its anchor"
    );

    let anchored_scalar = page.property("top-margin");
    let type_keyword = document
        .source_map
        .spans(&anchored_scalar, SchemaSpanKind::TypeKeyword)
        .first()
        .expect("type within anchored scalar");
    assert_eq!(&source[type_keyword.clone()], "number");
}

// ── Standalone declaration-parser parity ────────────────────────────────────
//
// A standalone pure payload must reach the same `parse_schema_declaration`
// authority an inline `$schema` value does. Before this parity existed the
// standalone path called `parse_yaml_schema`, which stores union arms as
// unchecked `SchemaArm::FileRef` — so an invalid or remote arm was silently
// accepted — while rejecting every scalar payload outright, so a valid
// whole-file reference was reported as a malformed document.

/// A scalar payload is a whole-file reference: a valid, active standalone
/// schema document that carries no inline schema.
#[test]
fn standalone_scalar_payload_is_an_active_whole_file_reference() {
    // The target deliberately does not exist: classification is passive, so a
    // valid declaration must not depend on the referenced file being readable.
    let source = "$schema: ./other.yaml\n";
    let document = parse_standalone_schema_document(source, Path::new("/w/schema.yaml"))
        .expect("a scalar reference is a valid declaration, not a malformed document")
        .expect("the pure envelope claims the document");

    assert_eq!(document.envelope, StandaloneSchemaEnvelope::Pure);
    assert!(
        document.schema().is_none(),
        "a whole-file reference declares no inline schema"
    );
    let SchemaDeclaration::Reference(reference) = &document.declaration else {
        panic!("expected a reference declaration, got {:?}", document.declaration);
    };
    assert_eq!(reference.file_reference().raw(), "./other.yaml");
    assert_eq!(reference.kind(), SchemaReferenceKind::PathQualified);

    let root = SchemaSourcePath::root();
    let file_reference = document
        .source_map
        .spans(&root, SchemaSpanKind::FileReference)
        .first()
        .expect("the reference must be structurally located, not text-searched")
        .clone();
    assert_eq!(&source[file_reference.clone()], "./other.yaml");
    assert_eq!(file_reference, 9..21);

    let declaration = document
        .source_map
        .spans(&root, SchemaSpanKind::Declaration)
        .first()
        .expect("declaration span")
        .clone();
    assert_eq!(&source[declaration], "./other.yaml");
}

/// Every rejected pure payload the declaration parser refuses, with the
/// authored substring the diagnostic must be able to range.
#[test]
fn standalone_pure_payloads_are_rejected_by_the_declaration_parser() {
    // (label, source, the smallest offending authored substring)
    let cases: [(&str, &str, &str); 5] = [
        ("whitespace-only scalar", "$schema: \"   \"\n", "\"   \""),
        ("whitespace-only arm", "$schema: [\"   \"]\n", "\"   \""),
        (
            "remote arm",
            "$schema: [https://example.com/schema.yaml]\n",
            "https://example.com/schema.yaml",
        ),
        (
            "remote scalar",
            "$schema: https://example.com/schema.yaml\n",
            "https://example.com/schema.yaml",
        ),
        (
            // The valid first arm must not launder the invalid second one.
            "mixed valid/invalid union",
            "$schema:\n  - ./valid.yaml\n  - https://example.com/schema.yaml\n",
            "https://example.com/schema.yaml",
        ),
    ];

    for (label, source, offending) in cases {
        let error = parse_standalone_schema_document(source, Path::new("/w/schema.yaml"))
            .expect_err(&format!("{label}: must be rejected, not silently accepted"));
        assert!(
            matches!(error, SchemaError::SchemaDocument { .. }),
            "{label}: a recognized envelope reports a schema-document error, got {error:?}"
        );
        assert!(
            source.contains(offending),
            "{label}: fixture must contain the offending substring"
        );
    }
}

/// The inverse of the scalar-payload rule: a *tagged* envelope's `types` is a
/// named-type mapping, never a whole-file reference.
#[test]
fn tagged_types_payload_stays_mapping_only() {
    for source in ["kind: schema\ntypes: ./other.yaml\n", "kind: schema\ntypes: []\n"] {
        let error = parse_standalone_schema_document(source, Path::new("/w/schema.yaml"))
            .expect_err("a tagged `types` payload must be a mapping");
        assert!(matches!(error, SchemaError::SchemaDocument { .. }), "{error:?}");
    }
}

/// A valid root union of references stays valid and carries no inline schema
/// arms it did not author.
#[test]
fn standalone_root_union_of_valid_references_is_accepted() {
    let source = "$schema:\n  - ./a.yaml\n  - ./b.yaml\n";
    let document = parse_standalone_schema_document(source, Path::new("/w/schema.yaml"))
        .expect("valid local reference arms")
        .expect("the pure envelope claims the document");
    let schema = document.schema().expect("a root union is an inline schema");
    let SchemaDeclaration::Schema(_) = &document.declaration else {
        panic!("a sequence payload is a schema declaration");
    };
    assert!(matches!(schema, SimplifiedSchema::Union(arms) if arms.len() == 2));
}
