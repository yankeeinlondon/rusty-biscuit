//! An unknown key inside a union's object arm is reported as the ordinary
//! unknown-key problem of a closed object, not as the generic "not valid
//! under any of the schemas" union failure.
//!
//! The key is reported only when every arm that admits the value's type
//! rejects it at that node, so a key another arm declares, or one an open
//! mapping admits, is never labeled stray.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure, ValidationProblem, ValidationProblemCode};

#[track_caller]
fn schema(document: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(document).unwrap_or_else(|error| panic!("schema: {error}"))
}

/// A schema whose one property `v` has the type `type_text`.
#[track_caller]
fn property_schema(type_text: &str) -> SimplifiedSchema {
    schema(&format!("$schema:\n  v: '{type_text}'\n"))
}

#[track_caller]
fn problems(schema: &SimplifiedSchema, value: &Value) -> Vec<ValidationProblem> {
    match schema.validate_strict(value) {
        Err(ValidationFailure::InvalidData { problems }) => problems,
        other => panic!("expected invalid data for {value}, got {other:?}"),
    }
}

fn unknown_keys(problems: &[ValidationProblem]) -> Vec<(String, Vec<String>)> {
    problems
        .iter()
        .filter(|problem| problem.code == ValidationProblemCode::UnknownKey)
        .map(|problem| (problem.path.clone(), problem.offending_properties.clone()))
        .collect()
}

fn is_generic_union_failure(problem: &ValidationProblem) -> bool {
    problem.message.contains("is not valid under any of the schemas")
}

#[test]
fn an_unknown_key_in_the_object_arm_of_a_scalar_union_is_reported_at_the_object() {
    let schema = property_schema("boolean | { a: number, b: number }");

    let found = problems(&schema, &json!({ "v": { "a": 1, "extra": 2 } }));

    assert_eq!(found.len(), 1, "{found:?}");
    let problem = &found[0];
    assert_eq!(problem.code, ValidationProblemCode::UnknownKey);
    assert_eq!(problem.message, "Additional properties are not allowed ('extra' was unexpected)");
    assert_eq!(problem.instance_path.segments(), ["v"]);
    assert_eq!(problem.offending_property.as_deref(), Some("extra"));
    assert_eq!(problem.offending_properties, ["extra"]);
}

#[test]
fn the_union_problem_has_the_shape_of_a_plain_closed_object_problem() {
    let plain = problems(
        &schema("$schema:\n  v: '{ a: number, b: number }'\n"),
        &json!({ "v": { "a": 1, "x": 1, "y": 2 } }),
    );
    let union = problems(
        &property_schema("boolean | { a: number, b: number }"),
        &json!({ "v": { "a": 1, "x": 1, "y": 2 } }),
    );

    assert_eq!(plain.len(), 1, "{plain:?}");
    assert_eq!(union.len(), 1, "{union:?}");
    assert_eq!(union[0].message, plain[0].message);
    assert_eq!(union[0].message, "Additional properties are not allowed ('x', 'y' were unexpected)");
    assert_eq!(union[0].path, plain[0].path);
    assert_eq!(union[0].kind, plain[0].kind);
    assert_eq!(union[0].offending_properties, plain[0].offending_properties);
    assert_eq!(union[0].offending_property, plain[0].offending_property);
}

#[test]
fn a_nullable_object_reports_its_unknown_key() {
    let schema = property_schema("{ a: number, b: number } | null");

    let found = problems(&schema, &json!({ "v": { "a": 1, "extra": 2 } }));

    assert_eq!(unknown_keys(&found), [("/v".to_string(), vec!["extra".to_string()])]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_deeper_failure_and_a_same_node_unknown_key_are_both_reported() {
    for type_text in ["boolean | { a: number, b: number }", "{ a: number, b: number } | null", "{ a: number, b: number }"] {
        let schema = property_schema(type_text);

        let found = problems(&schema, &json!({ "v": { "a": "x", "extra": 2 } }));

        assert_eq!(unknown_keys(&found), [("/v".to_string(), vec!["extra".to_string()])], "`{type_text}`: {found:?}");
        assert!(
            found.iter().any(|problem| problem.path == "/v/a" && problem.code == ValidationProblemCode::TypeMismatch),
            "`{type_text}`: {found:?}"
        );
        assert_eq!(found.len(), 2, "`{type_text}`: {found:?}");
    }
}

#[test]
fn a_key_another_object_arm_declares_is_never_labeled_stray() {
    let schema = property_schema("{ a: number } | { b: number }");

    assert!(schema.validate_strict(&json!({ "v": { "b": 1 } })).is_ok());
    let found = problems(&schema, &json!({ "v": { "a": 1, "b": 1 } }));

    assert!(unknown_keys(&found).is_empty(), "{found:?}");
    assert!(found.iter().any(is_generic_union_failure), "{found:?}");
}

#[test]
fn a_key_every_object_arm_rejects_is_reported_alone() {
    let schema = property_schema("{ a: number } | { b: number }");

    let found = problems(&schema, &json!({ "v": { "a": 1, "zzz": 1 } }));

    assert_eq!(unknown_keys(&found), [("/v".to_string(), vec!["zzz".to_string()])]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_literal_arm_does_not_veto_an_unknown_key() {
    let schema = property_schema("literal(auto) | { a: number }");

    let found = problems(&schema, &json!({ "v": { "a": 1, "zzz": 1 } }));

    assert_eq!(unknown_keys(&found), [("/v".to_string(), vec!["zzz".to_string()])]);
}

#[test]
fn a_union_reached_through_a_named_union_type_reports_the_key() {
    let schema = schema(concat!(
        "kind: schema\n",
        "types:\n",
        "  settings:\n",
        "    provider: string\n",
        "    voice: voices\n",
        "  voices:\n",
        "    - string\n",
        "    - gendered\n",
        "  gendered:\n",
        "    male: string\n",
        "    female: string\n",
        "$schema:\n",
        "  tts:\n",
        "    - boolean\n",
        "    - settings\n",
    ));

    let at_settings = problems(&schema, &json!({ "tts": { "provider": "say", "bogus": 1 } }));
    let at_voice = problems(&schema, &json!({ "tts": { "voice": { "male": "a", "extra": "b" } } }));

    assert_eq!(unknown_keys(&at_settings), [("/tts".to_string(), vec!["bogus".to_string()])]);
    assert_eq!(unknown_keys(&at_voice), [("/tts/voice".to_string(), vec!["extra".to_string()])]);
}

#[test]
fn a_valid_value_without_an_extra_key_validates() {
    let schema = property_schema("boolean | { a: number, b: number }");

    assert!(schema.validate_strict(&json!({ "v": { "a": 1, "b": 2 } })).is_ok());
    assert!(schema.validate_strict(&json!({ "v": true })).is_ok());
}

#[test]
fn an_open_mapping_arm_admits_the_key() {
    let open = property_schema("boolean | { a: number, <string>: string }");
    let beside_closed = property_schema("{ a: number } | { b: number, <string>: string }");

    assert!(open.validate_strict(&json!({ "v": { "a": 1, "zzz": "y" } })).is_ok());
    let wrong_value = problems(&open, &json!({ "v": { "a": 1, "zzz": 3 } }));
    let admitted_by_one_arm = problems(&beside_closed, &json!({ "v": { "a": 1, "zzz": 3 } }));

    assert!(unknown_keys(&wrong_value).is_empty(), "{wrong_value:?}");
    assert!(wrong_value.iter().any(|problem| problem.path == "/v/zzz"), "{wrong_value:?}");
    assert!(unknown_keys(&admitted_by_one_arm).is_empty(), "{admitted_by_one_arm:?}");
}
