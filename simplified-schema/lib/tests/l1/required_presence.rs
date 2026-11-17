//! `required` is a presence contract only: it asks whether the key was
//! written, independently of whether the declared type admits `null`. These
//! tests drive the declaration table through the public strict and soft
//! APIs, the exported JSON Schema, the query view, named types, and the
//! positional tuple contract that `required` does not change.
//!
//! Default application is covered by `required_defaults.rs`.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure, ValidationProblemCode, ValidationSuccess};

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

/// The `(path, code)` of each problem of a failed validation.
#[track_caller]
fn codes(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Vec<(String, ValidationProblemCode)> {
    match result {
        Err(failure) => failure.problems().iter().map(|problem| (problem.path.clone(), problem.code)).collect(),
        Ok(success) => panic!("expected a failure, got {success:?}"),
    }
}

/// Asserts `input` is accepted as written by strict and soft validation.
#[track_caller]
fn accepted_as_written(schema: &SimplifiedSchema, input: &Value, label: &str) {
    for (mode, result) in [("strict", schema.validate_strict(input)), ("soft", schema.validate_soft(input))] {
        match result {
            Ok(success) => {
                assert!(!success.is_modified(), "{label} ({mode}): unexpectedly modified: {success:?}");
                assert_eq!(success.value.as_ref(), input, "{label} ({mode})");
            }
            Err(failure) => panic!("{label} ({mode}): expected acceptance, got {:?}", failure.problems()),
        }
    }
}

/// Asserts strict and soft validation fail `input` with exactly `expected`.
/// A missing required key is reported at its parent object's pointer.
#[track_caller]
fn refused(schema: &SimplifiedSchema, input: &Value, expected: (&str, ValidationProblemCode), label: &str) {
    let expected = vec![(expected.0.to_string(), expected.1)];
    assert_eq!(codes(schema.validate_strict(input)), expected, "{label} (strict)");
    assert_eq!(codes(schema.validate_soft(input)), expected, "{label} (soft)");
}

const MISSING: ValidationProblemCode = ValidationProblemCode::MissingRequired;
const WRONG_TYPE: ValidationProblemCode = ValidationProblemCode::TypeMismatch;

#[test]
fn an_optional_property_accepts_a_missing_key_and_a_written_null() {
    let optional = schema("$schema:\n  n: number\n");
    accepted_as_written(&optional, &json!({}), "missing");
    accepted_as_written(&optional, &json!({ "n": null }), "null");
}

#[test]
fn a_required_number_needs_its_key_and_rejects_a_written_null_as_a_wrong_type() {
    let required = schema("$schema:\n  n: number(required)\n");
    refused(&required, &json!({}), ("", MISSING), "missing");
    refused(&required, &json!({ "n": null }), ("/n", WRONG_TYPE), "null");
    accepted_as_written(&required, &json!({ "n": 3 }), "a number");
}

#[test]
fn a_required_null_admitting_property_needs_its_key_and_accepts_a_written_null() {
    for declaration in ["n: null(required)", "n: unknown(required)", "n: any(required)", "n: [number(required), null]"] {
        let required = schema(&format!("$schema:\n  {declaration}\n"));
        refused(&required, &json!({}), ("", MISSING), declaration);
        accepted_as_written(&required, &json!({ "n": null }), declaration);
    }
    // The union's other arm still judges a non-null value.
    let union = schema("$schema:\n  n: [number(required), null]\n");
    accepted_as_written(&union, &json!({ "n": 3 }), "a number");
    refused(&union, &json!({ "n": "x" }), ("/n", WRONG_TYPE), "a string");
}

#[test]
fn a_null_arm_does_not_cancel_a_marker_hoisted_from_another_arm() {
    for declaration in ["n: string(required) | null", "n: null | string(required)", "n: string(required) | unknown"] {
        let required = schema(&format!("$schema:\n  {declaration}\n"));
        assert!(required.query("n").unwrap().unwrap().is_required(), "{declaration}");
        let exported = required.to_json_schema().unwrap();
        assert_eq!(exported["required"], json!(["n"]), "{declaration}");
        refused(&required, &json!({}), ("", MISSING), declaration);
        accepted_as_written(&required, &json!({ "n": null }), declaration);
    }
}

#[test]
fn the_query_view_reports_presence_independently_of_null_admission() {
    let declared = schema(
        "$schema:\n  a: null(required)\n  b: unknown(required)\n  c: null\n  d: unknown\n  e: string(required)\n",
    );
    let required = |name: &str| declared.query(name).unwrap().unwrap().is_required();
    assert!(required("a") && required("b") && required("e"));
    assert!(!required("c") && !required("d"));
}

#[test]
fn a_named_type_that_admits_null_keeps_a_use_site_marker() {
    let types = "types:\n  Maybe: string | null\n";
    let marked = schema(&format!("$schema:\n  n: Maybe(required)\n{types}"));
    assert!(marked.query("n").unwrap().unwrap().is_required());
    assert_eq!(marked.to_json_schema().unwrap()["required"], json!(["n"]));
    refused(&marked, &json!({}), ("", MISSING), "missing");
    accepted_as_written(&marked, &json!({ "n": null }), "null");
    accepted_as_written(&marked, &json!({ "n": "x" }), "a string");

    let unmarked = schema(&format!("$schema:\n  n: Maybe\n{types}"));
    assert!(!unmarked.query("n").unwrap().unwrap().is_required());
    accepted_as_written(&unmarked, &json!({}), "unmarked missing");
}

#[test]
fn a_marker_inherited_from_a_named_types_field_survives_null_admission() {
    let declared = schema("$schema:\n  owner: Person(required)\ntypes:\n  Person:\n    note: null(required)\n");
    refused(&declared, &json!({ "owner": {} }), ("/owner", MISSING), "missing field");
    accepted_as_written(&declared, &json!({ "owner": { "note": null } }), "written null");
}

#[test]
fn nested_objects_and_list_items_apply_the_same_presence_rule() {
    let declared = schema("$schema:\n  rows: '{ id: unknown(required) }[]'\n");
    refused(&declared, &json!({ "rows": [{ "id": 1 }, {}] }), ("/rows/1", MISSING), "missing in item");
    accepted_as_written(&declared, &json!({ "rows": [{ "id": null }] }), "null in item");
}

#[test]
fn tuple_positions_keep_their_positional_contract() {
    // An optional position may be left out, but a written `null` needs a
    // null-admitting position type; `required` on the tuple is the
    // property's presence, not its items'.
    let declared = schema("$schema:\n  range: 'tuple([number, number?])'\n  open: 'tuple([number, (number | null)?])'\n");
    accepted_as_written(&declared, &json!({ "range": [1] }), "omitted trailing position");
    accepted_as_written(&declared, &json!({ "range": [1, 2] }), "both positions");
    assert!(declared.validate_strict(&json!({ "range": [1, null] })).is_err(), "a null optional position");
    assert!(declared.validate_soft(&json!({ "range": [1, null] })).is_err(), "a null optional position (soft)");
    accepted_as_written(&declared, &json!({ "open": [1, null] }), "a null-admitting position");
    accepted_as_written(&declared, &json!({}), "the tuple property itself is optional");
}

#[test]
fn a_recursive_named_type_that_admits_null_keeps_a_use_site_marker() {
    // `List` is stored once as a definition and referenced; its null arm
    // does not strip the marker at either use.
    let declared = schema(
        "$schema:\n  head: List(required)\n  tail: List\ntypes:\n  List: Cell | null\n  Cell:\n    value: number\n    next: List(required)\n",
    );
    assert_eq!(declared.to_json_schema().unwrap()["required"], json!(["head"]));
    refused(&declared, &json!({}), ("", MISSING), "missing head");
    accepted_as_written(&declared, &json!({ "head": null }), "a null head");
    accepted_as_written(&declared, &json!({ "head": { "value": 1, "next": null } }), "one cell");
    refused(&declared, &json!({ "head": { "value": 1 } }), ("/head", MISSING), "a cell without `next`");
}
