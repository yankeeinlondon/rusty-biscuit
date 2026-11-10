//! `ConversionError::located_failures` / `coercion::locate_failures`: one
//! record per failing location, flattened through union failures.
//!
//! Each `as` is lowered as the property `v` of a schema and bound by the
//! coercion engine, as a host binding data to a contract does. The optional
//! property adds a `null` arm, which the flattening drops like any other.

use serde_json::{Value, json};
use simplified_schema::coercion::{
    ActualKind, ConversionError, ExpectedType, FailureReason, bind, locate_failures,
};
use simplified_schema::{SimplifiedSchema, shape_from_json_schema};

#[track_caller]
fn bind_error(as_text: &str, value: &Value) -> ConversionError {
    let text = format!("$schema:\n  v: '{as_text}'\n");
    let schema = SimplifiedSchema::try_from(text.as_str()).unwrap_or_else(|error| panic!("`{as_text}`: {error}"));
    let json = schema.to_json_schema().expect("lowers");
    let shape = shape_from_json_schema(&json["properties"]["v"]);
    bind(&shape, value).expect_err("the value does not bind")
}

/// `(pointer, expected, actual)` of each record.
#[track_caller]
fn records(as_text: &str, value: Value) -> Vec<(String, String, ActualKind)> {
    let error = bind_error(as_text, &value);
    let located = error.located_failures();
    assert_eq!(located, locate_failures(&error.failures), "the method and the slice helper agree");
    located
        .into_iter()
        .map(|record| (record.pointer, record.expected.to_string(), record.actual))
        .collect()
}

fn row(pointer: &str, expected: &str, actual: ActualKind) -> (String, String, ActualKind) {
    (pointer.to_string(), expected.to_string(), actual)
}

#[test]
fn a_plain_array_reports_each_failing_element() {
    assert_eq!(
        records("number[]", json!(["macOS", 2, "3", true, null])),
        [
            row("/0", "number", ActualKind::String),
            row("/3", "number", ActualKind::Boolean),
            row("/4", "number", ActualKind::Null),
        ]
    );
}

#[test]
fn a_nullable_array_never_lists_the_null_arm() {
    assert_eq!(
        records("number[] | null", json!(["macOS", "2", "x"])),
        [row("/0", "number", ActualKind::String), row("/2", "number", ActualKind::String)]
    );
    // No conversion reaches the array arm: one record at the union.
    assert_eq!(records("number[] | null", json!({ "a": 1 })), [row("", "number[]", ActualKind::Object)]);
}

#[test]
fn tuple_rows_report_missing_slots_and_extra_elements() {
    assert_eq!(
        records("tuple([string, number])[]", json!([["a", "1"], ["b", "x"], ["c"], ["d", "4", "extra"]])),
        [
            row("/1/1", "number", ActualKind::String),
            row("/2/1", "number", ActualKind::Missing),
            row("/3/2", "absent", ActualKind::String),
        ]
    );
}

#[test]
fn object_rows_report_extra_keys_as_absent_and_skip_converted_cells() {
    let value = json!([
        { "name": "a", "port": "8080" },
        { "name": "b", "port": "eighty" },
        { "name": "c", "port": "" },
        { "name": "d", "port": " 42 " },
        { "name": "e" },
        { "name": "f", "port": "1", "extra": "z" },
        { "name": "g", "port": "x" },
    ]);
    let found = records("{ name: string, port: number }[]", value);
    let pointers: Vec<(&str, &str, ActualKind)> =
        found.iter().map(|(pointer, expected, actual)| (pointer.as_str(), expected.as_str(), *actual)).collect();
    assert_eq!(
        pointers,
        [
            ("/1/port", "number", ActualKind::String),
            ("/2/port", "number", ActualKind::String),
            ("/5/extra", "absent", ActualKind::String),
            ("/6/port", "number", ActualKind::String),
        ]
    );
}

#[test]
fn two_reachable_container_arms_are_one_record_at_the_union() {
    let error = bind_error("{a: string}[] | {b: number}[]", &json!([{ "b": "1" }, { "b": "x" }, { "b": "y" }]));
    let located = error.located_failures();
    assert_eq!(located.len(), 1, "{located:?}");
    let record = &located[0];
    assert_eq!(record.pointer, "");
    assert_eq!(record.actual, ActualKind::Array);
    let ExpectedType::Union(arms) = &record.expected else {
        panic!("expected the remaining arms, got {:?}", record.expected);
    };
    assert_eq!(arms.len(), 2, "{arms:?}");
    assert!(arms.iter().all(|arm| *arm != "null"), "{arms:?}");
    assert!(matches!(record.failure.reason, FailureReason::NoOptionAccepts { .. }));

    assert_eq!(records("number[] | boolean[]", json!(["x", "true"])).len(), 1);
}

#[test]
fn a_per_element_union_reports_each_element() {
    let found = records("({a: string} | {b: number})[]", json!([{ "b": "1" }, { "b": "x" }, { "c": "z" }, "scalar"]));
    let shapes: Vec<(&str, ActualKind)> = found.iter().map(|(pointer, _, actual)| (pointer.as_str(), *actual)).collect();
    assert_eq!(shapes, [("/1", ActualKind::Object), ("/2", ActualKind::Object), ("/3", ActualKind::String)]);
    assert!(found.iter().all(|(_, expected, _)| expected.contains(" | ")), "{found:?}");
}

#[test]
fn the_one_reachable_arm_is_descended_into() {
    assert_eq!(records("string[] | {a: string}", json!({ "a": [1] })), [row("/a", "string", ActualKind::Array)]);
    assert_eq!(records("string[] | {a: string}", json!({ "b": 1 })), [row("/b", "absent", ActualKind::Number)]);
    let at_root = records("string[] | {a: string}", json!(42));
    assert_eq!(at_root.len(), 1, "{at_root:?}");
    assert_eq!(at_root[0].0, "");
    assert_eq!(at_root[0].2, ActualKind::Number);
    assert!(at_root[0].1.starts_with("string[] | "), "{at_root:?}");
}

#[test]
fn pointers_escape_slash_tilde_and_the_empty_key() {
    let found = records("{ <string>: number }", json!({ "a/b": "x", "c~d": "y", "": "z", "ok": "1" }));
    let pointers: Vec<&str> = found.iter().map(|(pointer, _, _)| pointer.as_str()).collect();
    assert_eq!(pointers, ["/a~1b", "/c~0d", "/"]);
}

#[test]
fn an_ambiguous_union_is_one_record() {
    let error = bind_error("{ a: number, b: boolean } | { a: boolean, b: number }", &json!({ "a": "1", "b": "1" }));
    let located = error.located_failures();
    assert_eq!(located.len(), 1, "{located:?}");
    assert!(matches!(located[0].failure.reason, FailureReason::AmbiguousUnion { .. }), "{located:?}");
}
