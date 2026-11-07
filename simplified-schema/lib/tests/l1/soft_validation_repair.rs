//! `validate_soft` reports only the real problems when a value has
//! convertible siblings beside an unexpected object key or tuple element.
//!
//! An unexpected entry used to keep the whole top-level value unconverted,
//! so the complete schema then judged text such as `"8080"` against
//! `number` and reported it. The diagnostic copy now keeps every conversion
//! and every unexpected entry, so only the unexpected entry is reported, and
//! the result is still a failure.

use serde_json::{Value, json};
use simplified_schema::coerce::coerce_frontmatter;
use simplified_schema::{SimplifiedSchema, ValidationFailure};

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema builds: {error}\n{text}"))
}

/// `(path, offending key)` of every problem soft validation reports.
#[track_caller]
fn soft_problems(schema: &SimplifiedSchema, value: &Value) -> Vec<(String, Option<String>, String)> {
    let Err(ValidationFailure::InvalidData { problems }) = schema.validate_soft(value) else {
        panic!("soft validation of {value} must fail with invalid data");
    };
    problems
        .into_iter()
        .map(|problem| (problem.path, problem.offending_property, problem.message))
        .collect()
}

#[test]
fn an_extra_key_in_one_row_is_the_only_problem() {
    let schema = schema("$schema:\n  rows: '{ name: string, port: number }[]'\n");
    let value = json!({ "rows": [
        { "name": "a", "port": "8080" },
        { "name": "b", "port": "1", "extra": "z" },
    ] });
    let problems = soft_problems(&schema, &value);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, "/rows/1", "{problems:?}");
    assert_eq!(problems[0].1.as_deref(), Some("extra"), "{problems:?}");

    // Control: without the extra key the rows convert and are accepted.
    let clean = json!({ "rows": [{ "name": "a", "port": "8080" }, { "name": "b", "port": "1" }] });
    let accepted = schema.validate_soft(&clean).expect("converts");
    assert_eq!(accepted.value["rows"], json!([{ "name": "a", "port": 8080 }, { "name": "b", "port": 1 }]));
}

#[test]
fn an_extra_tuple_element_is_the_only_problem() {
    let schema = schema("$schema:\n  pairs: 'tuple([string, number])[]'\n");
    let value = json!({ "pairs": [["a", "1"], ["b", "4", "extra"]] });
    let problems = soft_problems(&schema, &value);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, "/pairs/1/2", "{problems:?}");
}

#[test]
fn an_extra_key_in_a_nested_object_keeps_its_siblings_converted() {
    let schema = schema("$schema:\n  servers: '{ meta: { port: number }, count: number }[]'\n  total: number\n");
    let value = json!({
        "servers": [
            { "meta": { "port": "80" }, "count": "2" },
            { "meta": { "port": "81", "zone": "eu" }, "count": "3" },
        ],
        "total": "5",
    });
    let problems = soft_problems(&schema, &value);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].0, "/servers/1/meta", "{problems:?}");
    assert_eq!(problems[0].1.as_deref(), Some("zone"), "{problems:?}");
}

#[test]
fn a_failing_value_beside_an_extra_key_is_reported_with_it() {
    let schema = schema("$schema:\n  rows: '{ name: string, port: number }[]'\n");
    let value = json!({ "rows": [
        { "name": "a", "port": "8080" },
        { "name": "b", "port": "eighty", "extra": "z" },
    ] });
    let mut paths: Vec<String> = soft_problems(&schema, &value).into_iter().map(|(path, _, _)| path).collect();
    paths.sort();
    assert_eq!(paths, ["/rows/1", "/rows/1/port"]);
}

#[test]
fn a_nullable_container_reports_only_the_extra_key() {
    let schema = schema("$schema:\n  rows: '{ name: string, port: number }[] | null'\n");
    let value = json!({ "rows": [{ "name": "a", "port": "8080" }, { "name": "b", "extra": "z" }] });
    let problems = soft_problems(&schema, &value);
    assert!(!problems.is_empty());
    assert!(problems.iter().all(|(path, _, _)| !path.ends_with("/port")), "{problems:?}");
    assert!(problems.iter().any(|(_, key, _)| key.as_deref() == Some("extra")), "{problems:?}");
}

/// Two reachable arms explain nothing on their own, so nothing is set
/// aside below the union; the value still fails.
#[test]
fn unions_that_no_single_arm_explains_still_fail() {
    let arms = schema("$schema:\n  rows: '{a: string}[] | {b: number}[]'\n");
    let value = json!({ "rows": [{ "b": "1", "c": 1 }] });
    assert!(matches!(arms.validate_soft(&value), Err(ValidationFailure::InvalidData { .. })));

    let ambiguous = schema("$schema:\n  v: '{ a: number, b: boolean } | { a: boolean, b: number }'\n");
    let value = json!({ "v": { "a": "1", "b": "1" } });
    assert!(matches!(ambiguous.validate_soft(&value), Err(ValidationFailure::InvalidData { .. })));
}

/// Write-back conversion commits only what the complete schema accepts:
/// the rows with an extra key stay as written, while a clean sibling
/// converts.
#[test]
fn write_back_never_commits_a_partially_converted_value() {
    let schema = schema("$schema:\n  rows: '{ name: string, port: number }[]'\n  count: number\n");
    let json_schema = schema.to_json_schema().expect("lowers");
    let value = json!({ "rows": [{ "name": "a", "port": "8080", "extra": 1 }], "count": "3" });
    let outcome = coerce_frontmatter(&json_schema, &value);
    assert_eq!(outcome.value, json!({ "rows": [{ "name": "a", "port": "8080", "extra": 1 }], "count": 3 }));
}
