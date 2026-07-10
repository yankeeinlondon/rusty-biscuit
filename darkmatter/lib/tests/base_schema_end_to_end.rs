use std::path::Path;

use darkmatter::markdown::{
    Markdown,
    schemas::{DarkmatterSchemas, darkmatter_base_json_schema, darkmatter_base_schema, parse_yaml_schema, to_json_schema},
};
use serde_yaml_ng::Value as YamlValue;

fn base_api() -> DarkmatterSchemas {
    DarkmatterSchemas::new()
        .with_baseline(darkmatter_base_schema())
        .expect("base schema must be baseline-compatible")
}

fn claudine_schema_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas/claudine.yaml")
}

fn validate_with_base(frontmatter: &str) -> darkmatter::markdown::schemas::ValidationReport {
    let md: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
    base_api().validate(&md).expect("validation must run")
}

fn parse_schema_yaml(raw: &str) -> serde_json::Value {
    let yaml: YamlValue = serde_yaml_ng::from_str(raw).expect("schema YAML must parse");
    let schema = parse_yaml_schema(&yaml).expect("SimplifiedSchema must parse");
    to_json_schema(&schema).expect("SimplifiedSchema must convert")
}

#[test]
fn base_schema_file_parses_and_converts() {
    let schema = darkmatter_base_schema();
    let json = darkmatter_base_json_schema();

    match schema {
        darkmatter::markdown::schemas::SimplifiedSchema::Single(shape) => {
            assert!(shape.properties.contains_key("$schema"));
            assert!(shape.properties.contains_key("title"));
            assert!(shape.properties.contains_key("ctx"));
        }
        other => panic!("base schema must be a single object shape, got {other:?}"),
    }

    assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("object"));
    assert!(
        json.get("properties")
            .and_then(|v| v.as_object())
            .is_some_and(|properties| properties.contains_key("ctx")),
        "base JSON Schema must expose ctx"
    );
}

#[test]
fn claudine_schema_file_resolves_as_baseline() {
    DarkmatterSchemas::new()
        .with_baseline_from_file(claudine_schema_path())
        .expect("Claudine schema must resolve imports and remain baseline-compatible");

    let raw = std::fs::read_to_string(claudine_schema_path()).expect("schema must be readable");
    let yaml: YamlValue = serde_yaml_ng::from_str(&raw).expect("schema YAML must parse");
    let properties = yaml
        .get("$schema")
        .and_then(YamlValue::as_mapping)
        .expect("schema must declare a property mapping");
    assert!(properties.contains_key("max_turns"));
    assert!(!properties.contains_key("maxTurns"));
}

#[test]
fn claudine_schema_accepts_typed_lifecycle_stack() {
    let md: Markdown = "---\nstart:\n  info: Starting\n  stack:\n    - when: env.CI == 'true'\n      action:\n        - message: Build started\n        - action: shell\n          command: cargo test\n          no_error: true\n---\nBody\n".into();
    let report = DarkmatterSchemas::new()
        .with_baseline_from_file(claudine_schema_path())
        .expect("Claudine schema must load")
        .validate(&md)
        .expect("validation must run");

    assert!(
        report.valid,
        "valid lifecycle stack must pass: {:?}",
        report.problems
    );
}

#[test]
fn claudine_schema_rejects_malformed_lifecycle_stack() {
    let md: Markdown = "---\nstart:\n  stack: run-now\n---\nBody\n".into();
    let report = DarkmatterSchemas::new()
        .with_baseline_from_file(claudine_schema_path())
        .expect("Claudine schema must load")
        .validate(&md)
        .expect("validation must run");

    assert!(!report.valid, "a lifecycle stack must be an array");
}

#[test]
fn claudine_schema_accepts_loop_controls_and_lifecycle_concerns_together() {
    let md: Markdown = "---\nloop:\n  while: step < 3\n  action:\n    op: increment\n    prop: step\n  info: Iteration complete\n  stack:\n    - action: stop\n---\nBody\n".into();
    let report = DarkmatterSchemas::new()
        .with_baseline_from_file(claudine_schema_path())
        .expect("Claudine schema must load")
        .validate(&md)
        .expect("validation must run");

    assert!(
        report.valid,
        "loop controls and lifecycle concerns must compose: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_validation_accepts_valid_examples_and_unknown_keys() {
    let report = validate_with_base(
        "title: Release Notes\n\
         description: Human-readable summary\n\
         tags: [release, darkmatter]\n\
         draft: false\n\
         custom_key: 42\n",
    );

    assert!(
        report.valid,
        "valid base frontmatter plus unknown keys must pass: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_validation_rejects_invalid_known_property_values() {
    let report = validate_with_base("title: 42\ndraft: maybe\n");

    assert!(
        !report.valid,
        "invalid known property values must fail validation"
    );
    let diagnostics = report
        .problems
        .iter()
        .map(|problem| {
            format!(
                "{} {}",
                problem.path,
                problem.property.as_deref().unwrap_or(&problem.message)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        diagnostics.contains("title") || diagnostics.contains("draft"),
        "diagnostics should identify known invalid properties: {diagnostics}"
    );
}

#[test]
fn document_schema_definitions_override_baseline_properties() {
    let md: Markdown = "---\n$schema:\n  title: number\ntitle: 42\n---\nBody\n".into();
    let report = base_api().validate(&md).expect("validation must run");

    assert!(
        report.valid,
        "document schema should override baseline title type: {:?}",
        report.problems
    );
}

#[test]
fn schema_document_transcludes_same_file_as_library_source() {
    let doc_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/schemas/darkmatter-schema.md")
        .canonicalize()
        .expect("schema docs path must exist");
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/schemas/darkmatter.yaml")
        .canonicalize()
        .expect("schema YAML path must exist");
    let doc = std::fs::read_to_string(&doc_path).expect("schema docs must be readable");

    assert!(
        doc.contains("::code ./darkmatter.yaml"),
        "schema docs must transclude the neighboring schema source"
    );
    let transclusion_path = doc_path
        .parent()
        .expect("schema docs must have a parent")
        .join("darkmatter.yaml")
        .canonicalize()
        .expect("transclusion target must exist");
    assert_eq!(transclusion_path, schema_path);

    let file_json = {
        let raw = std::fs::read_to_string(&schema_path).expect("schema YAML must be readable");
        let frontmatter: YamlValue = serde_yaml_ng::from_str(&raw).expect("schema YAML must parse");
        let schema_value = frontmatter
            .get("$schema")
            .expect("schema YAML must contain $schema");
        to_json_schema(&parse_yaml_schema(schema_value).expect("schema YAML must parse"))
            .expect("schema YAML must convert")
    };
    assert_eq!(file_json, darkmatter_base_json_schema());
}

#[test]
fn nested_mapping_object_matches_inline_object_literal_json_schema() {
    let mapping = parse_schema_yaml("item:\n  foo: string\n  bar: number\n");
    let inline = parse_schema_yaml("item: \"{ foo: string, bar: number }\"\n");

    assert_eq!(mapping, inline);
}

#[test]
fn sequence_union_arms_accept_nested_mapping_object_shapes() {
    let md: Markdown = "---\n$schema:\n  choice:\n    - string\n    - kind: string(required)\n      value: number\nchoice:\n  kind: score\n  value: 10\n---\nBody\n".into();
    let report = DarkmatterSchemas::new()
        .validate(&md)
        .expect("validation must run");

    assert!(
        report.valid,
        "mapping object union arm must validate: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_ctx_is_darkmatter_owned_generated_context() {
    let report = validate_with_base("title: Context Example\n");
    assert!(
        report.valid,
        "authored frontmatter may omit ctx: {:?}",
        report.problems
    );

    let generated_ctx = validate_with_base("ctx:\n  today: 2026-07-04\n");
    assert!(
        generated_ctx.valid,
        "known generated ctx keys must pass when present: {:?}",
        generated_ctx.problems
    );

    let unknown_ctx = validate_with_base("ctx:\n  project_slug: biscuit\n");
    assert!(
        !unknown_ctx.valid,
        "custom user ctx keys are not part of the base schema"
    );

    let json = darkmatter_base_json_schema();
    let ctx = &json["properties"]["ctx"];
    let ctx_object = ctx
        .get("anyOf")
        .and_then(|any_of| any_of.as_array())
        .and_then(|arms| arms.iter().find(|arm| arm["type"].as_str() == Some("object")))
        .unwrap_or(ctx);
    assert_eq!(ctx_object["type"].as_str(), Some("object"));
    assert_eq!(ctx_object["additionalProperties"].as_bool(), Some(false));
    assert!(
        ctx_object["properties"]["today"]["x-darkmatter-generated"].as_bool() == Some(true),
        "base ctx must expose generated metadata for known runtime keys: {ctx_object:?}"
    );
}
