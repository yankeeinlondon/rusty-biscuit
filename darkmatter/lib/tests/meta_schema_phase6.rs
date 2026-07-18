//! Phase 6 public parser-product coverage for standalone schema authoring.

use std::path::Path;

use darkmatter::markdown::schemas::{
    SchemaSourcePath, SchemaSpanKind, StandaloneSchemaEnvelope,
    parse_standalone_schema_document,
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
