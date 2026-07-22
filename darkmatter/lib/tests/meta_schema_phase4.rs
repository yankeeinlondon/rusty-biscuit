//! Phase 4 contracts for semantic meta-type JSON Schema lowering, validation,
//! coercion preservation, trigger matching, and shipped-artifact compatibility.

use std::{fs, path::Path};

use darkmatter::markdown::{
    Markdown,
    schemas::{
        Constraint, DarkmatterSchemas, PropertyAtom, SimplifiedType, TypeExpr,
        ValidationProblemCode, parse_yaml_schema, to_json_schema,
        triggers::{MatchExpr, matches as trigger_matches},
    },
};
use serde_json::{Value, json};

fn compiled_schema(source: &str) -> Value {
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).expect("schema YAML");
    let schema = parse_yaml_schema(&yaml).expect("SimplifiedSchema");
    to_json_schema(&schema).expect("compiled JSON Schema")
}

fn validate_document(source: &str) -> darkmatter::markdown::schemas::ValidationReport {
    DarkmatterSchemas::new()
        .validate(&Markdown::from(source))
        .expect("schema compilation and validation")
}

fn semantic_match(name: &str, ty: SimplifiedType, is_array: bool, value: Value) -> bool {
    let expr = MatchExpr::Property {
        name: name.to_string(),
        atom: PropertyAtom {
            ty: TypeExpr::Primitive(ty),
            is_array,
            constraints: vec![Constraint::Required],
            array_constraints: Vec::new(),
            description: None,
        },
    };
    trigger_matches(&expr, &json!({ name: value }), "docs/input.md")
}

#[test]
fn semantic_fragments_lower_with_carriers_keywords_and_nullable_wrappers() {
    let schema = compiled_schema(concat!(
        "required_definition: type-definition(required)\n",
        "optional_definition: type-definition\n",
        "required_schema: schema(required)\n",
        "optional_schema: schema\n",
    ));
    let properties = schema["properties"].as_object().expect("properties");
    assert_eq!(
        properties["required_definition"],
        json!({
            "type": ["string", "object", "array"],
            "x-darkmatter-type-definition": true,
        })
    );
    assert_eq!(
        properties["optional_definition"],
        json!({
            "anyOf": [
                { "type": "null" },
                {
                    "type": ["string", "object", "array"],
                    "x-darkmatter-type-definition": true,
                },
            ]
        })
    );
    assert_eq!(
        properties["required_schema"],
        json!({
            "type": ["string", "object", "array"],
            "x-darkmatter-schema": true,
        })
    );
    assert_eq!(
        properties["optional_schema"],
        json!({
            "anyOf": [
                { "type": "null" },
                {
                    "type": ["string", "object", "array"],
                    "x-darkmatter-schema": true,
                },
            ]
        })
    );
}

#[test]
fn semantic_array_lowering_validates_each_independent_item() {
    let schema = compiled_schema(concat!(
        "definitions: type-definition[](required)\n",
        "schemas: schema[](required)\n",
    ));
    assert_eq!(
        schema["properties"]["definitions"],
        json!({
            "type": "array",
            "items": {
                "type": ["string", "object", "array"],
                "x-darkmatter-type-definition": true,
            },
        })
    );
    assert_eq!(
        schema["properties"]["schemas"],
        json!({
            "type": "array",
            "items": {
                "type": ["string", "object", "array"],
                "x-darkmatter-schema": true,
            },
        })
    );
}

#[test]
fn semantic_keyword_failures_are_structured_and_distinguishable() {
    for (semantic_type, candidate, keyword) in [
        (
            "type-definition",
            "string(min(nope))",
            "x-darkmatter-type-definition",
        ),
        ("schema", "https://example.com/schema.yaml", "x-darkmatter-schema"),
    ] {
        let source = format!(
            "---\n$schema:\n  candidate: {semantic_type}(required)\ncandidate: {candidate}\n---\nBody\n"
        );
        let report = validate_document(&source);
        assert!(!report.valid, "{semantic_type} candidate must fail");
        let problem = report.problems.first().expect("one semantic problem");
        assert_eq!(problem.code, ValidationProblemCode::ConstraintViolation);
        assert_eq!(problem.path, "/candidate");
        assert_eq!(problem.instance_path.segments(), &["candidate"]);
        assert_eq!(problem.line, Some(4));
        assert_eq!(problem.column, Some(1));
        let schema_path = problem.schema_path.as_ref().expect("keyword schema path");
        assert_eq!(schema_path.segments().last().map(String::as_str), Some(keyword));
    }
}

#[test]
fn semantic_carriers_are_validation_and_compose_no_ops() {
    let source = concat!(
        "---\n$schema:\n",
        "  definition: type-definition(required)\n",
        "  declaration: schema(required)\n",
        "  definitions: type-definition[](required)\n",
        "definition: { title: string(required) }\n",
        "declaration: [./missing.yaml, { kind: literal(review) }]\n",
        "definitions: [string, [literal(auto), number]]\n",
        "---\nBody\n",
    );
    let original = Markdown::from(source);
    let before = original.frontmatter().as_map().clone();
    let report = DarkmatterSchemas::new()
        .validate(&original)
        .expect("validation must run");
    assert!(report.valid, "semantic carriers must validate: {:?}", report.problems);
    assert_eq!(original.frontmatter().as_map(), &before, "validation is read-only");

    let (composed, _) = original.compose().expect("normal compose path");
    assert_eq!(composed.frontmatter().as_map(), &before);
}

#[test]
fn semantic_types_match_triggers_by_passive_parse() {
    for value in [
        json!("string(required)"),
        json!({ "title": "string(required)" }),
        json!(["literal(auto)", { "width": "number(required)" }]),
    ] {
        assert!(semantic_match("definition", SimplifiedType::TypeDefinition, false, value));
    }
    for value in [json!(true), json!("string(min(nope))"), json!([])] {
        assert!(!semantic_match("definition", SimplifiedType::TypeDefinition, false, value));
    }

    for value in [
        json!("./schemas/does-not-exist.yaml"),
        json!({ "title": "string(required)" }),
        json!(["./missing.yaml", { "kind": "literal(review)" }]),
    ] {
        assert!(semantic_match("declaration", SimplifiedType::Schema, false, value));
    }
    for value in [json!(true), json!("https://example.com/schema.yaml"), json!([])] {
        assert!(!semantic_match("declaration", SimplifiedType::Schema, false, value));
    }

    assert!(semantic_match(
        "definitions",
        SimplifiedType::TypeDefinition,
        true,
        json!(["string", ["literal(auto)", "number"]]),
    ));
    assert!(!semantic_match(
        "definitions",
        SimplifiedType::TypeDefinition,
        true,
        json!(["string", true]),
    ));
}

#[test]
fn shipped_schema_artifacts_validate_through_semantic_keywords() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/schemas");
    let mut declarations = Vec::new();
    let mut definitions = Vec::new();
    let mut paths = fs::read_dir(&root)
        .expect("schema directory")
        .map(|entry| entry.expect("schema entry").path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let source = fs::read_to_string(&path).expect("shipped schema artifact");
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).expect("shipped YAML");
        let Some(map) = value.as_mapping() else { continue };
        if let Some(declaration) = map.get(serde_yaml_ng::Value::String("$schema".into())) {
            declarations.push(declaration.clone());
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("env.yaml")
            && let Some(catalog) = map
                .get(serde_yaml_ng::Value::String("types".into()))
                .and_then(serde_yaml_ng::Value::as_mapping)
        {
            definitions.extend(catalog.values().cloned());
        }
    }
    assert!(!declarations.is_empty(), "shipped schema declarations");
    assert!(!definitions.is_empty(), "shipped type definitions");

    let declarations = serde_yaml_ng::to_string(&declarations).expect("serialize declarations");
    let definitions = serde_yaml_ng::to_string(&definitions).expect("serialize definitions");
    let source = format!(
        "---\n$schema:\n  declarations: schema[](required)\n  definitions: type-definition[](required)\ndeclarations:\n{}definitions:\n{}---\nBody\n",
        declarations.lines().map(|line| format!("  {line}\n")).collect::<String>(),
        definitions.lines().map(|line| format!("  {line}\n")).collect::<String>(),
    );
    let report = validate_document(&source);
    assert!(report.valid, "shipped artifacts must validate: {:?}", report.problems);
}
