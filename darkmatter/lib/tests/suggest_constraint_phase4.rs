use std::path::Path;

use darkmatter::markdown::schemas::{
    SchemaError, StandaloneSchemaEnvelope, parse_standalone_schema_document,
    resolve::resolve_schema, to_json_schema,
};
use serde_json::json;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, contents).unwrap();
}

fn resolve_file(path: &Path) -> darkmatter::markdown::schemas::resolve::ResolvedSchema {
    resolve_schema(
        &json!(path.to_string_lossy()),
        path.parent().unwrap_or_else(|| Path::new(".")),
    )
    .unwrap()
}

#[test]
fn content_classifier_recognizes_only_the_two_envelopes() {
    let pure = "$schema:\n  name: string(suggest(Bob, Mary))\n";
    let tagged = "kind: schema\ntypes:\n  name: string(suggest(Bob, Mary))\n";
    let pure_document = parse_standalone_schema_document(pure, "pure.yaml")
        .unwrap()
        .unwrap();
    let tagged_document = parse_standalone_schema_document(tagged, "tagged.yaml")
        .unwrap()
        .unwrap();
    assert_eq!(pure_document.envelope, StandaloneSchemaEnvelope::Pure);
    assert_eq!(tagged_document.envelope, StandaloneSchemaEnvelope::Tagged);
    assert_eq!(
        to_json_schema(pure_document.schema().expect("mapping payload")).unwrap(),
        to_json_schema(tagged_document.schema().expect("mapping payload")).unwrap()
    );

    for ordinary in [
        "name: string\n",
        "$schema: https://json-schema.org/draft/2020-12/schema\ntype: object\n",
        "$schema:\n  name: string\ndescription: ordinary yaml\n",
        "kind: other\ntypes:\n  name: string\n",
    ] {
        assert!(
            parse_standalone_schema_document(ordinary, "ordinary.yaml")
                .unwrap()
                .is_none(),
            "ordinary YAML must not be claimed: {ordinary}"
        );
    }
}

#[test]
fn claimed_malformed_envelopes_are_schema_document_errors() {
    for source in [
        "kind: schema\n",
        "kind: schema\ntypes: []\n",
        "kind: schema\ntypes: {}\ndescription: unsupported\n",
        "kind: schema\ntypes: {}\nextra: value\n",
        // A tagged `types` payload is a named-type mapping, so — unlike a pure
        // payload — a reference scalar is still malformed here.
        "kind: schema\ntypes: ./other.yaml\n",
    ] {
        let error = parse_standalone_schema_document(source, "claimed.yaml").unwrap_err();
        assert!(
            matches!(error, SchemaError::SchemaDocument { .. }),
            "recognized malformed envelope must not fall back: {error:?}"
        );
    }
}

/// A pure scalar payload is a whole-file reference, parsed by the shared
/// declaration authority — so it accepts exactly the strings an inline
/// `$schema` scalar accepts, including a bare name.
///
/// `$schema: string` reads like a type name, but the declaration authority has
/// no notion of type names at declaration position; it is a bare-name schema
/// reference resolved against the schema roots, and rejecting it here would put
/// standalone parsing back out of parity with inline parsing.
#[test]
fn pure_scalar_payloads_are_whole_file_references() {
    for source in ["$schema: string\n", "$schema: ./other.yaml\n", "$schema: darkmatter.yaml\n"] {
        let document = parse_standalone_schema_document(source, "claimed.yaml")
            .expect("a scalar payload is a reference declaration")
            .expect("the pure envelope claims the document");
        assert!(document.schema().is_none(), "{source:?} declares no inline schema");
    }
}

#[test]
fn standalone_product_retains_path_spans_and_lints() {
    let source = "kind: schema\r\ntypes:\r\n  café: string(min(3); suggest(ok, valid))\r\n";
    let document = parse_standalone_schema_document(source, "schemas/café.yaml")
        .unwrap()
        .unwrap();
    assert_eq!(document.path, Path::new("schemas/café.yaml"));
    assert_eq!(document.suggestion_lints.len(), 1);
    let problem = &document.suggestion_lints[0];
    assert_eq!(problem.property, "café");
    assert_eq!(&source[problem.span.clone()], "ok");
}

#[test]
fn mapping_envelopes_resolve_identically_with_origin_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let pure = dir.path().join("pure.yaml");
    let tagged = dir.path().join("tagged.yaml");
    write(&pure, "$schema:\n  name: string(suggest(Bob, Mary))\n");
    write(
        &tagged,
        "kind: schema\ntypes:\n  name: string(suggest(Bob, Mary))\n",
    );

    let pure_resolved = resolve_file(&pure);
    let tagged_resolved = resolve_file(&tagged);
    assert_eq!(pure_resolved.json_schema, tagged_resolved.json_schema);
    assert_eq!(pure_resolved.referenced_files, vec![pure.canonicalize().unwrap()]);
    assert_eq!(tagged_resolved.referenced_files, vec![tagged.canonicalize().unwrap()]);
    assert_eq!(pure_resolved.origin.uri.as_deref(), Some(pure.as_path()));
    assert_eq!(tagged_resolved.origin.uri.as_deref(), Some(tagged.as_path()));
}

#[test]
fn pure_sequence_is_a_whole_file_union_but_not_a_namespace() {
    let dir = tempfile::tempdir().unwrap();
    let union = dir.path().join("union.yaml");
    write(
        &union,
        "$schema:\n  - person: string(suggest(Bob))\n  - team: string(suggest(Red))\n",
    );
    let resolved = resolve_file(&union);
    assert_eq!(resolved.json_schema["anyOf"].as_array().unwrap().len(), 2);

    let consumer = dir.path().join("consumer.yaml");
    write(&consumer, "$schema:\n  value: person@./union.yaml\n");
    let error = resolve_schema(&json!(consumer.to_string_lossy()), dir.path()).unwrap_err();
    assert!(matches!(error, SchemaError::SchemaDocument { .. }));
}

#[test]
fn named_imports_share_pure_and_tagged_mapping_namespaces() {
    let dir = tempfile::tempdir().unwrap();
    let pure = dir.path().join("pure.yaml");
    let tagged = dir.path().join("tagged.yaml");
    write(&pure, "$schema:\n  name: string(suggest(Bob, Mary))\n");
    write(
        &tagged,
        "kind: schema\ntypes:\n  name: string(suggest(Bob, Mary))\n",
    );
    let pure_consumer = dir.path().join("pure-consumer.yaml");
    let tagged_consumer = dir.path().join("tagged-consumer.yaml");
    write(&pure_consumer, "$schema:\n  value: name@./pure.yaml\n");
    write(
        &tagged_consumer,
        "$schema:\n  value: name@./tagged.yaml\n",
    );

    let pure_resolved = resolve_file(&pure_consumer);
    let tagged_resolved = resolve_file(&tagged_consumer);
    assert_eq!(pure_resolved.json_schema, tagged_resolved.json_schema);
    assert_eq!(pure_resolved.imports, vec![pure.canonicalize().unwrap()]);
    assert_eq!(tagged_resolved.imports, vec![tagged.canonicalize().unwrap()]);
}

#[test]
fn tagged_schema_resolves_nested_imports_and_examples_from_its_directory() {
    let dir = tempfile::tempdir().unwrap();
    let schema = dir.path().join("schemas/main.yaml");
    let types = dir.path().join("schemas/nested/types.yaml");
    let example = dir.path().join("examples/today.yaml");
    write(&types, "$schema:\n  token: string(suggest(alpha, beta))\n");
    write(
        &example,
        "kind: example\ninvocation: today\nreturns: 2026-07-10\ndescription: date\n",
    );
    write(
        &schema,
        "kind: schema\ntypes:\n  value: token@./nested/types.yaml\n  today: date(example(../examples/today.yaml))\n",
    );

    let resolved = resolve_file(&schema);
    assert_eq!(resolved.imports, vec![types.canonicalize().unwrap()]);
    assert_eq!(resolved.examples, vec![example.canonicalize().unwrap()]);
}

#[test]
fn import_cycles_remain_bounded_across_envelope_forms() {
    let dir = tempfile::tempdir().unwrap();
    let pure = dir.path().join("pure.yaml");
    let tagged = dir.path().join("tagged.yaml");
    write(&pure, "$schema:\n  node: node@./tagged.yaml\n");
    write(
        &tagged,
        "kind: schema\ntypes:\n  node: node@./pure.yaml\n",
    );
    let error = resolve_schema(&json!(pure.to_string_lossy()), dir.path()).unwrap_err();
    assert!(matches!(error, SchemaError::ImportCycle { .. }));
}

#[test]
fn raw_json_schema_remains_distinct_and_cannot_supply_named_imports() {
    let dir = tempfile::tempdir().unwrap();
    let raw_yaml = dir.path().join("raw.yaml");
    write(
        &raw_yaml,
        "$schema: https://json-schema.org/draft/2020-12/schema\ntype: object\nproperties:\n  value:\n    type: string\n    x-darkmatter-suggest: [hidden]\n",
    );
    let resolved = resolve_file(&raw_yaml);
    assert!(resolved.simplified.is_none());
    assert_eq!(
        resolved.json_schema["properties"]["value"]["x-darkmatter-suggest"],
        json!(["hidden"])
    );

    let consumer = dir.path().join("consumer.yaml");
    write(&consumer, "$schema:\n  value: value@./raw.yaml\n");
    let error = resolve_schema(&json!(consumer.to_string_lossy()), dir.path()).unwrap_err();
    assert!(matches!(error, SchemaError::AmbiguousReferenced { .. }));
}
