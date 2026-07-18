//! Phase 5 contracts for the shipped Darkmatter base-schema migration.

use std::fs;

use darkmatter::markdown::{
    Markdown,
    schemas::{
        DarkmatterSchemas, ValidationProblemCode, darkmatter_base_json_schema,
        resolve::resolve_schema_with_roots,
    },
};
use serde_json::{Value, json};

fn property_contains(value: &Value, property: &str, key: &str) -> bool {
    let Some(property_schema) = value.get("properties").and_then(|value| value.get(property)) else {
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

fn shipped_baseline() -> darkmatter::markdown::schemas::EffectiveSchema {
    let api = DarkmatterSchemas::new()
        .with_darkmatter_baseline_json_schema()
        .expect("shipped Darkmatter base schema");
    api.effective_for(&Markdown::from("---\ntitle: Seed\n---\nBody\n"))
        .expect("baseline preparation")
        .expect("baseline schema")
}

#[test]
fn shipped_base_schema_declares_schema_semantics_and_precise_raw_json_wording() {
    let source = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas/darkmatter.yaml"),
    )
    .expect("real shipped base-schema artifact");
    let declaration = source
        .lines()
        .find(|line| line.trim_start().starts_with("\"$schema\":"))
        .expect("$schema property declaration");
    assert!(declaration.contains("\"schema ->"), "declaration was: {declaration}");
    assert!(
        declaration.contains("Raw JSON Schema is supported through referenced files"),
        "description must keep raw JSON Schema on the referenced-file boundary: {declaration}"
    );
    assert!(
        !declaration.contains("inline schema, referenced schema file, root union, or raw JSON Schema"),
        "description must not imply raw JSON Schema is an inline mapping form"
    );

    let compiled = darkmatter_base_json_schema();
    assert!(property_contains(&compiled, "$schema", "x-darkmatter-schema"));
}

#[test]
fn shipped_base_schema_preserves_all_resolver_accepted_declaration_forms() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("simplified.yaml"),
        "$schema:\n  title: string(required)\n",
    )
    .expect("write SimplifiedSchema fixture");
    fs::write(
        dir.path().join("raw.json"),
        r#"{"type":"object","properties":{"title":{"type":"string"}}}"#,
    )
    .expect("write raw JSON Schema fixture");

    for (form, declaration) in [
        ("inline", json!({ "title": "string(required)" })),
        ("referenced SimplifiedSchema", json!("./simplified.yaml")),
        (
            "root union",
            json!(["./simplified.yaml", { "kind": "literal(review)" }]),
        ),
        ("referenced raw JSON Schema", json!("./raw.json")),
    ] {
        let resolved = resolve_schema_with_roots(&declaration, dir.path(), &[])
            .unwrap_or_else(|error| panic!("{form} must remain accepted: {error}"));
        assert!(resolved.json_schema.is_object(), "{form} must produce a JSON Schema");
    }
}

#[test]
fn malformed_declarations_fail_baseline_validation_before_resolver_preparation() {
    let effective = shipped_baseline();
    for (form, declaration, expected_code) in [
        ("native boolean", json!(true), ValidationProblemCode::TypeMismatch),
        ("native number", json!(42), ValidationProblemCode::TypeMismatch),
        (
            "empty root union",
            json!([]),
            ValidationProblemCode::ConstraintViolation,
        ),
        (
            "invalid root-union arm",
            json!([{ "title": "string" }, true]),
            ValidationProblemCode::ConstraintViolation,
        ),
    ] {
        let instance = json!({ "$schema": declaration });
        let report = effective.validate(&instance);
        assert!(!report.valid, "{form} must fail at baseline validation");
        let problem = report
            .problems
            .iter()
            .find(|problem| problem.path == "/$schema")
            .unwrap_or_else(|| panic!("{form} must report /$schema: {:?}", report.problems));
        assert_eq!(problem.code, expected_code);
        if expected_code == ValidationProblemCode::ConstraintViolation {
            assert!(
                problem.message.contains("schema declaration")
                    || problem.message.contains("SimplifiedSchema"),
                "{form} must report the grammar-specific failure: {}",
                problem.message
            );
        }

        assert!(
            resolve_schema_with_roots(&instance["$schema"], std::path::Path::new("."), &[])
                .is_err(),
            "the later resolver must continue rejecting {form}"
        );
    }

    assert!(effective.validate(&json!({})).valid, "missing optional $schema remains valid");
    assert!(
        effective.validate(&json!({ "$schema": Value::Null })).valid,
        "an optional null retains Darkmatter's established missing-value semantics"
    );
    assert!(
        resolve_schema_with_roots(&Value::Null, std::path::Path::new("."), &[]).is_err(),
        "a present null still cannot be prepared as a schema declaration"
    );
    assert!(
        effective.validate(&json!({ "$schema": "true" })).valid,
        "a quoted YAML scalar is local-reference syntax, unlike native true"
    );
}
