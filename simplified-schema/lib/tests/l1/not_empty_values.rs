//! `not-empty` through the public API: a written value must have content.
//! For each type family that has an "empty" (text, `yaml`/`json`, lists,
//! objects and mappings, declarations) the empty value is rejected under
//! strict and soft validation, while a missing key, an explicit `null` on an
//! optional property, and a non-empty value are accepted. On a list it is
//! the same bound as `min(1)`. On a type with no "empty" it is a schema
//! error.
//!
//! The removed `non-null` spellings are refused in `declaration_grammar.rs`.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure};

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

fn one_property(declaration: &str) -> SimplifiedSchema {
    schema(&format!("$schema:\n  p: {declaration}\n"))
}

/// Asserts `{ p: value }` (or `{}` for `None`) is accepted by strict and soft
/// validation.
#[track_caller]
fn accepts(schema: &SimplifiedSchema, value: Option<Value>, label: &str) {
    let input = value.map_or_else(|| json!({}), |value| json!({ "p": value }));
    assert!(schema.validate_strict(&input).is_ok(), "{label}: strict refused {input}: {:?}", schema.validate_strict(&input));
    assert!(schema.validate_soft(&input).is_ok(), "{label}: soft refused {input}");
}

/// Asserts `{ p: value }` is refused by strict and soft validation, with a
/// problem located at or below `/p`.
#[track_caller]
fn refuses(schema: &SimplifiedSchema, value: Value, label: &str) {
    let input = json!({ "p": value });
    for (mode, result) in [("strict", schema.validate_strict(&input)), ("soft", schema.validate_soft(&input))] {
        match result {
            Err(failure @ ValidationFailure::InvalidData { .. }) => {
                assert!(
                    failure.problems().iter().any(|problem| problem.path.starts_with("/p")),
                    "{label} ({mode}): {:?}",
                    failure.problems()
                );
            }
            other => panic!("{label} ({mode}): expected `{input}` refused as data, got {other:?}"),
        }
    }
}

/// The lowering error a declaration raises, whether construction or
/// export reports it.
#[track_caller]
fn schema_error(declaration: &str) -> String {
    let text = format!("$schema:\n  p: {declaration}\n");
    match SimplifiedSchema::try_from(text.as_str()) {
        Err(error) => error.to_string(),
        Ok(schema) => match schema.to_json_schema() {
            Err(error) => error.to_string(),
            Ok(lowered) => panic!("`{declaration}` lowered: {lowered}"),
        },
    }
}

#[test]
fn text_rejects_empty_and_blank_strings() {
    for declaration in ["string(not-empty)", "yaml(not-empty)", "json(not-empty)"] {
        let declared = one_property(declaration);
        refuses(&declared, json!(""), declaration);
        refuses(&declared, json!("  \n\t"), declaration);
        accepts(&declared, None, declaration);
        accepts(&declared, Some(Value::Null), declaration);
    }
    accepts(&one_property("string(not-empty)"), Some(json!("x")), "string");
    accepts(&one_property("yaml(not-empty)"), Some(json!("a: 1")), "yaml");
    accepts(&one_property("json(not-empty)"), Some(json!("{\"a\": 1}")), "json");
}

#[test]
fn a_yaml_value_converted_from_empty_text_is_still_empty() {
    // Soft conversion never turns blank text into content: an interpolation
    // that produced nothing stays refused.
    let declared = one_property("yaml(not-empty)");
    assert!(declared.validate_soft(&json!({ "p": "" })).is_err());
    // A native mapping is serialized to YAML text with content.
    assert!(declared.validate_soft(&json!({ "p": { "a": 1 } })).is_ok());
}

#[test]
fn a_list_rejects_an_empty_list_exactly_like_min_one() {
    let not_empty = one_property("string[](not-empty)");
    refuses(&not_empty, json!([]), "[]");
    accepts(&not_empty, None, "missing");
    accepts(&not_empty, Some(Value::Null), "null");
    accepts(&not_empty, Some(json!(["a"])), "one item");
    // Items keep their own type: an empty-string item is fine here.
    accepts(&not_empty, Some(json!([""])), "an empty item");

    let min_one = one_property("string[](min(1))");
    assert_eq!(not_empty.to_json_schema().unwrap(), min_one.to_json_schema().unwrap());
}

#[test]
fn a_list_bound_merges_with_min_and_max() {
    let merged = one_property("string[](not-empty; min(3))");
    assert_eq!(merged.to_json_schema().unwrap()["properties"]["p"]["anyOf"][1]["minItems"], json!(3));
    let message = schema_error("string[](not-empty; max(0))");
    assert!(message.contains("contradictory bounds") && message.contains("not-empty"), "{message}");
}

#[test]
fn item_and_list_constraints_stay_distinct() {
    let items = one_property("string(not-empty)[]");
    accepts(&items, Some(json!([])), "an empty list of non-empty strings");
    refuses(&items, json!([""]), "an empty item");
}

#[test]
fn objects_and_mappings_reject_an_empty_mapping() {
    for declaration in [
        "\"{ a: string }(not-empty)\"",
        "\"{ <string>: number }(not-empty)\"",
        "object(not-empty)",
        "schema(mapping; not-empty)",
    ] {
        let declared = one_property(declaration);
        refuses(&declared, json!({}), declaration);
        accepts(&declared, None, declaration);
        accepts(&declared, Some(Value::Null), declaration);
    }
    accepts(&one_property("\"{ a: string }(not-empty)\""), Some(json!({ "a": "x" })), "inline");
    accepts(&one_property("\"{ <string>: number }(not-empty)\""), Some(json!({ "a": 1 })), "dictionary");
    accepts(&one_property("object(not-empty)"), Some(json!({ "a": 1 })), "object");
    accepts(&one_property("schema(mapping; not-empty)"), Some(json!({ "a": "string" })), "mapping");
    // The declaration is still judged: a bad declaration is not "content".
    refuses(&one_property("schema(mapping; not-empty)"), json!({ "a": "nope(" }), "a malformed declaration");
}

#[test]
fn a_declaration_rejects_blank_text_and_empty_unions() {
    let declared = one_property("schema(not-empty)");
    refuses(&declared, json!(""), "empty text");
    refuses(&declared, json!("  "), "blank text");
    refuses(&declared, json!({}), "an empty mapping");
    refuses(&declared, json!([]), "an empty union");
    accepts(&declared, None, "missing");
    accepts(&declared, Some(Value::Null), "null");
    accepts(&declared, Some(json!({ "a": "string" })), "a mapping");
    accepts(&declared, Some(json!([{ "a": "string" }, { "b": "number" }])), "a union");
}

#[test]
fn types_without_an_empty_value_refuse_not_empty() {
    for declaration in ["number(not-empty)", "boolean(not-empty)", "null(not-empty)", "unknown(not-empty)"] {
        let message = schema_error(declaration);
        assert!(message.contains("`not-empty`") && message.contains("\"empty\" has no meaning"), "{declaration}: {message}");
    }
}

#[test]
fn a_required_not_empty_property_still_needs_its_key() {
    // `not-empty` never makes a key mandatory, and never accepts `null` for a
    // required non-null type.
    let declared = one_property("string(required; not-empty)");
    assert!(declared.validate_strict(&json!({})).is_err());
    assert!(declared.validate_strict(&json!({ "p": null })).is_err());
    assert!(declared.validate_strict(&json!({ "p": "" })).is_err());
    assert!(declared.validate_strict(&json!({ "p": "x" })).is_ok());
}

#[test]
fn a_not_empty_block_constraint_applies_to_its_object() {
    let declared = schema("$schema:\n  p:\n    a: string\n    $constraints:\n      not-empty: true\n");
    refuses(&declared, json!({}), "an empty object");
    accepts(&declared, Some(json!({ "a": "x" })), "a non-empty object");
    accepts(&declared, None, "missing");
}
