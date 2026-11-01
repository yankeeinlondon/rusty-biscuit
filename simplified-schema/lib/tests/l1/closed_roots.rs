//! `$constraints: { closed: true }` on a root, a root-union arm, and a nested
//! object, judged through the public `SimplifiedSchema` API.
//!
//! A closed root lowers to `additionalProperties: false`; its declared and
//! layered properties and its pattern-dictionary matches are admitted, and any
//! other key is an unknown-key problem reported at the key. A layer's closure
//! stays with that layer: a closed lower layer never closes the receiver.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure, ValidationProblem, ValidationProblemCode};

const CLOSED_ROOT: &str = "$schema:\n  name: string\n  $constraints: { closed: true }\n";
const OPEN_ROOT: &str = "$schema:\n  name: string\n";

#[track_caller]
fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

#[track_caller]
fn root_json(schema: &SimplifiedSchema) -> Value {
    schema.to_json_schema().expect("schema lowers").as_ref().clone()
}

/// The single problem both validation modes report for `value`, which must be
/// the same in each.
#[track_caller]
fn sole_problem(schema: &SimplifiedSchema, value: &Value) -> ValidationProblem {
    let mut seen = Vec::new();
    for (mode, result) in [("strict", schema.validate_strict(value)), ("soft", schema.validate_soft(value))] {
        match result {
            Err(ValidationFailure::InvalidData { mut problems }) => {
                assert_eq!(problems.len(), 1, "{mode}: {problems:#?}");
                seen.push(problems.remove(0));
            }
            other => panic!("{mode}: expected invalid data for {value}, got {other:?}"),
        }
    }
    let soft = seen.pop().expect("soft problem");
    let strict = seen.pop().expect("strict problem");
    assert_eq!(
        (&soft.path, soft.code, &soft.offending_property),
        (&strict.path, strict.code, &strict.offending_property),
        "both modes locate the problem alike"
    );
    strict
}

/// An unknown-key problem names the offending key and points at the object
/// that holds it.
#[track_caller]
fn assert_unknown_key(problem: &ValidationProblem, parent: &str, key: &str) {
    assert_eq!(problem.code, ValidationProblemCode::UnknownKey, "{problem:#?}");
    assert_eq!(problem.code.diagnostic_code(), simplified_schema::SCHEMA_UNKNOWN_KEY_CODE);
    assert_eq!(problem.path, parent, "{problem:#?}");
    assert_eq!(problem.offending_property.as_deref(), Some(key), "{problem:#?}");
}

#[track_caller]
fn assert_accepted(schema: &SimplifiedSchema, value: &Value) {
    for (mode, result) in [("strict", schema.validate_strict(value)), ("soft", schema.validate_soft(value))] {
        assert!(result.is_ok(), "{mode}: {value} must be accepted, got {result:?}");
    }
}

#[test]
fn an_open_root_accepts_an_undeclared_key() {
    let open = schema(OPEN_ROOT);
    assert_eq!(root_json(&open)["additionalProperties"], json!(true));
    assert_accepted(&open, &json!({"name": "Ada", "extra": 1}));
}

#[test]
fn a_closed_root_rejects_an_undeclared_key_at_the_key() {
    let closed = schema(CLOSED_ROOT);
    assert_accepted(&closed, &json!({"name": "Ada"}));
    let problem = sole_problem(&closed, &json!({"name": "Ada", "extra": 1}));
    assert_unknown_key(&problem, "", "extra");
    assert!(problem.instance_path.is_empty(), "{problem:#?}");
}

/// A closed root reports an extra key exactly as a nested object always has.
#[test]
fn a_closed_root_reports_like_a_nested_object() {
    let nested = schema("$schema:\n  box:\n    name: string\n");
    let problem = sole_problem(&nested, &json!({"box": {"name": "Ada", "extra": 1}}));
    assert_unknown_key(&problem, "/box", "extra");
}

#[test]
fn a_pattern_key_match_is_admitted_under_closure() {
    let closed = schema("$schema:\n  name: string\n  \"<starting::x->\": number\n  $constraints: { closed: true }\n");
    let lowered = root_json(&closed);
    assert_eq!(lowered["additionalProperties"], json!(false));
    assert!(lowered["patternProperties"].is_object(), "{lowered}");
    assert_accepted(&closed, &json!({"name": "Ada", "x-count": 3}));
    assert_unknown_key(&sole_problem(&closed, &json!({"name": "Ada", "other": 3})), "", "other");
}

/// The `<string>` catch-all admits every key, so closing its root changes
/// nothing: the catch-all keeps lowering to `additionalProperties`.
#[test]
fn a_catch_all_key_still_admits_every_key_under_closure() {
    let closed = schema("$schema:\n  name: string\n  \"<string>\": number\n  $constraints: { closed: true }\n");
    let open = schema("$schema:\n  name: string\n  \"<string>\": number\n");
    assert_eq!(root_json(&closed), root_json(&open));
    assert_eq!(root_json(&closed)["additionalProperties"], json!({"type": "number"}));
    assert_accepted(&closed, &json!({"name": "Ada", "anything": 3}));
}

/// The library merges declarations from other sources into a root only by
/// layering; closure is judged on the merged root, so a lower layer's
/// properties count as declared.
#[test]
fn a_layered_property_counts_as_declared_on_a_closed_receiver() {
    let mut closed = schema(CLOSED_ROOT);
    closed.layer_under(&[&schema("$schema:\n  owner: string\n")]).expect("layers");
    let lowered = root_json(&closed);
    assert_eq!(lowered["additionalProperties"], json!(false));
    assert!(lowered["properties"]["owner"].is_object(), "{lowered}");
    assert_accepted(&closed, &json!({"name": "Ada", "owner": "Bo"}));
    assert_unknown_key(&sole_problem(&closed, &json!({"name": "Ada", "extra": 1})), "", "extra");
}

#[test]
fn a_closed_arm_rejects_extras_while_an_open_sibling_accepts_them() {
    let union = schema(
        "$schema:\n  - kind: literal(a; required)\n    name: string\n    $constraints: { closed: true }\n  - kind: literal(b; required)\n    name: string\n",
    );
    let lowered = root_json(&union);
    let arms = lowered["anyOf"].as_array().expect("a root union");
    assert_eq!(arms[0]["additionalProperties"], json!(false));
    assert_eq!(arms[1]["additionalProperties"], json!(true));

    assert_accepted(&union, &json!({"kind": "a", "name": "Ada"}));
    assert_accepted(&union, &json!({"kind": "b", "name": "Ada", "extra": 1}));
    let problem = sole_problem(&union, &json!({"kind": "a", "name": "Ada", "extra": 1}));
    assert_unknown_key(&problem, "", "extra");
    assert_eq!(problem.arm_index, Some(0), "{problem:#?}");
}

#[test]
fn closure_survives_canonical_serialization() {
    let closed = schema(CLOSED_ROOT);
    let exported = closed.to_simplified_yaml().expect("exports");
    let reparsed = schema(&exported);
    assert_eq!(reparsed.to_simplified_yaml().expect("re-exports"), exported);
    assert_eq!(root_json(&reparsed), root_json(&closed));
    assert_unknown_key(&sole_problem(&reparsed, &json!({"name": "Ada", "extra": 1})), "", "extra");
}

#[test]
fn closure_exports_as_additional_properties_false_and_imports_back() {
    let closed = schema(CLOSED_ROOT);
    let lowered = root_json(&closed);
    assert_eq!(lowered["additionalProperties"], json!(false));

    let imported = schema(&lowered.to_string());
    assert_eq!(root_json(&imported)["additionalProperties"], json!(false));
    assert_accepted(&imported, &json!({"name": "Ada"}));
    assert_unknown_key(&sole_problem(&imported, &json!({"name": "Ada", "extra": 1})), "", "extra");
}

#[test]
fn a_closed_lower_layer_does_not_close_an_open_receiver() {
    let mut open = schema("$schema:\n  owner: string\n");
    open.layer_under(&[&schema(CLOSED_ROOT)]).expect("a closed lower layer is admitted");
    let lowered = root_json(&open);
    assert_eq!(lowered["additionalProperties"], json!(true));
    assert!(lowered["properties"]["name"].is_object(), "{lowered}");
    assert_accepted(&open, &json!({"owner": "Bo", "name": "Ada", "extra": 1}));
}

/// `layer_over` makes the incoming layers the higher-priority side; the merged
/// root keeps the closure of that side, so the open receiver's own
/// `additionalProperties: true` is what is replaced, not inherited.
#[test]
fn a_closed_higher_layer_keeps_its_closure_over_an_open_receiver() {
    let mut open = schema("$schema:\n  owner: string\n");
    open.layer_over(&[&schema(CLOSED_ROOT)]).expect("layers");
    assert_eq!(root_json(&open)["additionalProperties"], json!(false));
    assert_accepted(&open, &json!({"owner": "Bo", "name": "Ada"}));
    assert_unknown_key(&sole_problem(&open, &json!({"name": "Ada", "extra": 1})), "", "extra");
}

#[test]
fn nested_closed_is_a_no_op() {
    let plain = schema("$schema:\n  box:\n    name: string\n");
    let closed = schema("$schema:\n  box:\n    name: string\n    $constraints:\n      closed: true\n");
    assert_eq!(root_json(&closed), root_json(&plain));
    // An optional property lowers to `anyOf: [null, object]`.
    assert_eq!(root_json(&closed)["properties"]["box"]["anyOf"][1]["additionalProperties"], json!(false));
    for nested in [&plain, &closed] {
        assert_unknown_key(&sole_problem(nested, &json!({"box": {"name": "Ada", "extra": 1}})), "/box", "extra");
    }
}
