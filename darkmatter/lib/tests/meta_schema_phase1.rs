//! Executable contracts established in Phase 1 for the SimplifiedSchema
//! semantic meta-types.

use std::{fs, path::Path};

use biscuit_file::FileReference;
use darkmatter::markdown::{
    Markdown,
    schemas::{
        Constraint, DarkmatterSchemas, PropertyDef, SimplifiedSchema, TypeExpr,
        ValidationReport, darkmatter_base_json_schema, parse_standalone_schema_document,
        parse_yaml_schema, schema_type_descriptors,
        simplified::{grammar::MAX_INLINE_OBJECT_DEPTH, grammar::parse_type_expr,
            parse_yaml_schema_with_source, serialize_property_atom},
    },
};
use serde_json::{Value, json};

fn validate_candidate(semantic_type: &str, candidate_yaml: &str) -> ValidationReport {
    let source = format!(
        "---\n$schema:\n  candidate: {semantic_type}(required)\n{candidate_yaml}---\nBody\n"
    );
    DarkmatterSchemas::new()
        .validate(&Markdown::from(source.as_str()))
        .expect("the semantic schema must compile and validation must run")
}

fn assert_valid_candidate(semantic_type: &str, candidate_yaml: &str) {
    let report = validate_candidate(semantic_type, candidate_yaml);
    assert!(report.valid, "candidate should be valid: {:?}", report.problems);
}

fn assert_invalid_candidate(semantic_type: &str, candidate_yaml: &str) {
    let report = validate_candidate(semantic_type, candidate_yaml);
    assert!(!report.valid, "candidate should be rejected: {candidate_yaml:?}");
}

fn schema_property_contains(value: &Value, property: &str, key: &str) -> bool {
    let Some(property_schema) = value.get("properties").and_then(|v| v.get(property)) else {
        return false;
    };
    fn contains(value: &Value, key: &str) -> bool {
        match value {
            Value::Object(map) => {
                map.contains_key(key) || map.values().any(|value| contains(value, key))
            }
            Value::Array(items) => items.iter().any(|value| contains(value, key)),
            _ => false,
        }
    }
    contains(property_schema, key)
}

#[test]
fn type_definition_keyword_round_trips_and_reaches_the_catalog() {
    for keyword in ["type-definition", "type-definition[]"] {
        let atom = parse_type_expr("candidate", keyword).expect("keyword must parse");
        assert_eq!(serialize_property_atom(&atom), keyword);
    }
    assert!(
        schema_type_descriptors()
            .iter()
            .any(|descriptor| descriptor.keyword == "type-definition")
    );
}

#[test]
fn type_definition_validation_matches_the_property_definition_matrix() {
    for candidate in [
        "candidate: string(required)\n",
        "candidate: 'string(required)'\n",
        "candidate: \"string(required)\"\n",
        "candidate:\n  title: string(required)\n  metadata:\n    count: number\n",
        "candidate:\n  - literal(auto)\n  - number(min(1))\n  - width: number(required)\n",
    ] {
        assert_valid_candidate("type-definition", candidate);
    }

    for candidate in [
        "candidate: true\n",
        "candidate: 2\n",
        "candidate: null\n",
        "candidate: []\n",
        "candidate: [string, true]\n",
        "candidate: string(min(nope))\n",
        "",
    ] {
        assert_invalid_candidate("type-definition", candidate);
    }
}

#[test]
fn schema_validation_is_syntax_only_and_rejects_remote_references() {
    for reference in ["./schemas/does-not-exist.yaml", "does-not-exist.yaml"] {
        let parsed = FileReference::new(reference).expect("local reference syntax");
        assert_eq!(parsed.raw(), reference);
        assert_valid_candidate("schema", &format!("candidate: {reference}\n"));
    }
    for candidate in [
        "candidate:\n  title: string(required)\n",
        "candidate:\n  - ./schemas/does-not-exist.yaml\n  - kind: literal(review)\n",
    ] {
        assert_valid_candidate("schema", candidate);
    }

    for candidate in [
        "candidate: https://example.com/schema.yaml\n",
        "candidate: http://example.com/schema.yaml\n",
        "candidate: true\n",
        "candidate: 2\n",
        "candidate: null\n",
        "candidate: []\n",
        "candidate: [./schema.yaml, true]\n",
    ] {
        assert_invalid_candidate("schema", candidate);
    }
}

#[test]
fn semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips() {
    let source = concat!(
        "---\n$schema:\n",
        "  one_union: type-definition(required)\n",
        "  many_definitions: type-definition[](required)\n",
        "  one_schema_union: schema(required)\n",
        "  many_schemas: schema[](required)\n",
        "one_union: [string, number]\n",
        "many_definitions:\n  - string\n  - number\n  - [literal(auto), number]\n",
        "one_schema_union: [./missing-a.yaml, { title: string }]\n",
        "many_schemas:\n  - { title: string }\n  - [./missing-b.yaml, { kind: literal(review) }]\n",
        "---\nBody\n",
    );
    let original = Markdown::from(source);
    let report = DarkmatterSchemas::new().validate(&original).expect("validation must run");
    assert!(report.valid, "array/union matrix must validate: {:?}", report.problems);

    let expected = original.frontmatter().as_map().clone();
    let dir = tempfile::tempdir().expect("tempdir");
    let first_path = dir.path().join("first.md");
    let second_path = dir.path().join("second.md");
    fs::write(&first_path, original.as_string()).expect("first write");
    let first = Markdown::try_from(first_path.as_path()).expect("first read");
    assert_eq!(first.frontmatter().as_map(), &expected);
    fs::write(&second_path, first.as_string()).expect("second write");
    let second = Markdown::try_from(second_path.as_path()).expect("second read");
    assert_eq!(second.frontmatter().as_map(), &expected);
}

#[test]
fn existing_source_projection_covers_union_quote_crlf_and_utf8_variants() {
    let source = concat!(
        "plain: string(suggest(alpha, café))\r\n",
        "single: 'string(suggest(beta, ''café''))'\r\n",
        "double: \"string(suggest(gamma, 'caf\\u00e9'))\"\r\n",
        "union:\r\n",
        "  - string\r\n",
        "  - nested: string(suggest(delta, café))\r\n",
    );
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).expect("YAML");
    let schema = parse_yaml_schema_with_source(&yaml, source, 11).expect("source projection");
    let SimplifiedSchema::Single(shape) = schema else {
        panic!("expected one schema shape");
    };
    let mut atoms = Vec::new();
    for name in ["plain", "single", "double"] {
        let PropertyDef::Single(atom) = &shape.properties[name] else {
            panic!("expected a scalar definition for {name}");
        };
        atoms.push(atom);
    }
    let PropertyDef::Union(arms) = &shape.properties["union"] else {
        panic!("expected a property union");
    };
    let TypeExpr::InlineObject(nested) = &arms[1].ty else {
        panic!("expected a nested mapping union arm");
    };
    let PropertyDef::Single(nested) = &nested.properties["nested"] else {
        panic!("expected a nested scalar definition");
    };
    atoms.push(nested);

    for arm in atoms {
        let Constraint::Suggest(candidates) = arm
            .constraints
            .iter()
            .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
            .expect("suggestion constraint")
        else {
            unreachable!()
        };
        let span = candidates.last().expect("candidate").span.clone();
        let raw = &source[span.start - 11..span.end - 11];
        assert!(raw.contains("caf"), "projected authored span: {raw:?}");
    }
}

fn nested_mapping(depth: usize) -> String {
    assert!(depth > 0);
    let mut source = String::from("root:\n");
    for level in 0..depth - 1 {
        source.push_str(&"  ".repeat(level + 1));
        source.push_str(&format!("level_{level}:\n"));
    }
    source.push_str(&"  ".repeat(depth));
    source.push_str("leaf: string\n");
    source
}

#[test]
fn native_mapping_depth_uses_the_shared_structured_limit() {
    let at_limit: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&nested_mapping(MAX_INLINE_OBJECT_DEPTH)).expect("YAML");
    parse_yaml_schema(&at_limit).expect("the documented boundary must remain valid");

    let over_limit: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&nested_mapping(MAX_INLINE_OBJECT_DEPTH + 1)).expect("YAML");
    let error = parse_yaml_schema(&over_limit).expect_err("over-limit mapping must fail");
    let message = error.to_string();
    assert!(message.contains("nest") && message.contains(&MAX_INLINE_OBJECT_DEPTH.to_string()));
}

#[test]
fn shipped_schema_corpus_is_passively_classified_without_resolution() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas");
    let mut paths = fs::read_dir(&root)
        .expect("schema directory")
        .map(|entry| entry.expect("schema entry").path())
        .filter(|path| matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml")))
        .collect::<Vec<_>>();
    paths.sort();
    assert_eq!(paths.len(), 7, "every shipped YAML artifact must remain in the corpus");

    let mut schemas = Vec::new();
    let mut other_artifacts = Vec::new();
    let mut rejected_claims = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path).expect("read shipped artifact");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        match parse_standalone_schema_document(&source, &path) {
            Ok(Some(_)) => schemas.push(name),
            Ok(None) => other_artifacts.push(name),
            Err(_) => rejected_claims.push(name),
        }
    }
    assert_eq!(
        schemas,
        ["claudine-types.yaml", "claudine.yaml", "darkmatter.yaml", "env.yaml"]
    );
    assert_eq!(other_artifacts, ["expression-functions.yaml", "schema-definition.yaml"]);
    assert_eq!(rejected_claims, ["err.yaml"]);
}

#[test]
fn shipped_base_schema_retypes_schema_and_preserves_resolution_acceptance() {
    let compiled = darkmatter_base_json_schema();
    assert!(schema_property_contains(&compiled, "$schema", "x-darkmatter-schema"));

    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("simplified.yaml"), "$schema:\n  title: string(required)\n")
        .expect("write simplified schema");
    fs::write(
        dir.path().join("raw.json"),
        r#"{"type":"object","properties":{"title":{"type":"string"}}}"#,
    )
    .expect("write raw JSON Schema");

    let simplified_ref = FileReference::new("./simplified.yaml").expect("reference syntax");
    let raw_ref = FileReference::new("./raw.json").expect("reference syntax");
    for declaration in [
        json!({ "title": "string(required)" }),
        json!(simplified_ref.raw()),
        json!([simplified_ref.raw(), { "kind": "literal(review)" }]),
        json!(raw_ref.raw()),
    ] {
        darkmatter::markdown::schemas::resolve::resolve_schema_with_roots(
            &declaration,
            dir.path(),
            &[],
        )
        .expect("existing declaration shape must still resolve");
    }

    let api = DarkmatterSchemas::new()
        .with_darkmatter_baseline_json_schema()
        .expect("shipped base schema");
    let seed = Markdown::from("---\ntitle: Seed\n---\nBody\n");
    let effective = api
        .effective_for(&seed)
        .expect("baseline preparation")
        .expect("baseline schema");
    let malformed = json!({ "$schema": true });
    let report = effective.validate(&malformed);
    assert!(!report.valid, "validation-only use must reject malformed $schema early");
    assert!(
        darkmatter::markdown::schemas::resolve::resolve_schema_with_roots(
            &json!(true),
            dir.path(),
            &[],
        )
        .is_err(),
        "full preparation must continue rejecting the same malformed declaration",
    );
}
