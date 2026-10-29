//! Strict typed equality for `==` and `!=` through the public expression API
//! (`parse` + `evaluate`).
//!
//! Expected results are written out by hand, never computed by the comparator
//! under test. Large integers and unsigned values arrive as typed lookup
//! values, so parser rounding of literals cannot hide a comparison defect.

use std::collections::HashMap;

use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, parse};
use serde_json::{Number, Value, json};

struct Values(HashMap<&'static str, Value>);

impl EvaluationLookup for Values {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }
}

/// Typed values no literal can produce exactly. `missing` is deliberately
/// absent; `explicit_null` is present and `null`.
fn values() -> Values {
    Values(HashMap::from([
        ("explicit_null", Value::Null),
        ("two_pow_53", json!(9_007_199_254_740_992_i64)),
        ("two_pow_53_plus_one", json!(9_007_199_254_740_993_i64)),
        ("two_pow_53_float", json!(9_007_199_254_740_992.0_f64)),
        ("i64_min", json!(i64::MIN)),
        ("i64_min_float", json!(-9_223_372_036_854_775_808.0_f64)),
        ("i64_max", json!(i64::MAX)),
        ("i64_max_unsigned", Value::Number(Number::from(i64::MAX as u64))),
        ("u64_max", Value::Number(Number::from(u64::MAX))),
        ("i64_max_plus_one", Value::Number(Number::from(i64::MAX as u64 + 1))),
        ("two_pow_63_float", json!(9_223_372_036_854_775_808.0_f64)),
        ("negative_zero", json!(-0.0_f64)),
        ("negative_one", json!(-1)),
        ("text_five", json!("5")),
        ("object_ab", json!({"a": 1, "b": null})),
        ("object_a", json!({"a": 1})),
    ]))
}

fn eval(source: &str, lookup: &Values) -> Value {
    let expr = parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
    evaluate(&expr, lookup).unwrap_or_else(|error| panic!("{source}: {error}"))
}

/// Asserts `left == right` is `equal` and `left != right` its complement, in
/// both operand orders.
fn assert_pair(left: &str, right: &str, equal: bool, lookup: &Values) {
    for (a, b) in [(left, right), (right, left)] {
        assert_eq!(eval(&format!("({a}) == ({b})"), lookup), json!(equal), "{a} == {b}");
        assert_eq!(eval(&format!("({a}) != ({b})"), lookup), json!(!equal), "{a} != {b}");
    }
}

#[test]
fn the_motivating_table_now_compares_type_and_value() {
    let lookup = values();
    for (left, right, equal) in [
        ("5", "\"5\"", false),
        ("missing", "''", false),
        ("missing", "missing_too", true),
        ("[]", "'[]'", false),
        ("true", "\"true\"", false),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn the_decision_table_holds() {
    let lookup = values();
    for (left, right, equal) in [
        ("5", "number(\"5\")", true),
        ("5", "5.0", true),
        ("0", "false", false),
        ("'A'", "'a'", false),
        ("' a '", "'a'", false),
        ("[1, missing]", "[1.0, missing]", true),
        ("[1, 2]", "[2, 1]", false),
        ("{a: 1, b: missing}", "{b: missing, a: 1.0}", true),
        ("{a: missing}", "{}", false),
        ("[]", "{}", false),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn a_missing_value_is_null_and_null_equals_only_null() {
    let lookup = values();
    for (left, right, equal) in [
        ("missing", "explicit_null", true),
        ("explicit_null", "explicit_null", true),
        ("missing", "0", false),
        ("missing", "false", false),
        ("missing", "[]", false),
        ("missing", "{}", false),
        ("explicit_null", "''", false),
        ("explicit_null", "'null'", false),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn scalars_compare_only_within_their_runtime_type() {
    let lookup = values();
    for (left, right, equal) in [
        ("true", "true", true),
        ("false", "false", true),
        ("true", "false", false),
        ("1", "true", false),
        ("'abc'", "'abc'", true),
        ("''", "''", true),
        ("'abc'", "'abd'", false),
        ("5", "5.5", false),
        ("5", "text_five", false),
        ("number(text_five)", "5", true),
        ("'5'", "text_five", true),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn containers_compare_recursively() {
    let lookup = values();
    for (left, right, equal) in [
        ("[]", "[]", true),
        ("{}", "{}", true),
        ("[[]]", "[[], []]", false),
        ("[[1, {a: [2]}]]", "[[1.0, {a: [2.0]}]]", true),
        ("[[1, {a: [2]}]]", "[[1, {a: ['2']}]]", false),
        ("{a: {b: [missing]}}", "{a: {b: [explicit_null]}}", true),
        ("{a: {b: []}}", "{a: {b: {}}}", false),
        ("object_ab", "{b: missing, a: 1}", true),
        ("object_ab", "object_a", false),
        ("object_a", "{a: 1, b: 2}", false),
        ("['a', 'b']", "'[\"a\",\"b\"]'", false),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn numbers_compare_by_exact_value_across_storage() {
    let lookup = values();
    for (left, right, equal) in [
        ("two_pow_53", "two_pow_53_plus_one", false),
        ("two_pow_53", "two_pow_53_float", true),
        ("two_pow_53_plus_one", "two_pow_53_float", false),
        ("i64_min", "i64_min_float", true),
        ("i64_max", "i64_max_unsigned", true),
        ("i64_max", "i64_max_plus_one", false),
        ("i64_max_plus_one", "two_pow_63_float", true),
        ("u64_max", "u64_max", true),
        ("u64_max", "negative_one", false),
        ("u64_max", "i64_max", false),
        ("negative_zero", "0", true),
        ("negative_zero", "0.0", true),
        ("negative_zero", "-0", true),
        ("[negative_zero]", "[0]", true),
    ] {
        assert_pair(left, right, equal, &lookup);
    }
}

#[test]
fn ordering_still_coerces_while_equality_does_not() {
    let lookup = values();
    assert_eq!(eval("5 <= \"5\"", &lookup), json!(true));
    assert_eq!(eval("5 >= \"5\"", &lookup), json!(true));
    assert_eq!(eval("5 == \"5\"", &lookup), json!(false));
}

#[test]
fn an_operand_failure_stays_an_error_not_an_unequal_result() {
    let lookup = values();
    for source in ["(1 / 0) == 1", "5 != (5 % 0)", "no_such_function(1) == missing"] {
        let expr = parse(source).unwrap_or_else(|error| panic!("{source}: {error}"));
        assert!(evaluate(&expr, &lookup).is_err(), "{source} must fail");
    }
}

/// Reflexivity, symmetry, and the exact `!=` complement over every pair of a
/// sample corpus that spans each runtime type and storage form.
#[test]
fn equality_is_reflexive_symmetric_and_complemented_over_the_sample_corpus() {
    let lookup = values();
    let corpus = [
        "missing", "explicit_null", "true", "false", "0", "negative_zero", "1", "1.5", "5", "text_five",
        "''", "' '", "'a'", "'A'", "[]", "[missing]", "[1, 2]", "[2, 1]", "{}", "{a: missing}", "object_a",
        "object_ab", "two_pow_53", "two_pow_53_plus_one", "two_pow_53_float", "i64_min", "u64_max",
    ];
    let samples: Vec<Value> = corpus.iter().map(|source| eval(source, &lookup)).collect();
    for (index, left) in corpus.iter().enumerate() {
        assert_eq!(eval(&format!("{left} == {left}"), &lookup), json!(true), "{left} is reflexive");
        assert_eq!(eval(&format!("{left} != {left}"), &lookup), json!(false), "{left} is reflexive");
        for right in &corpus[index + 1..] {
            let forward = eval(&format!("{left} == {right}"), &lookup);
            let backward = eval(&format!("{right} == {left}"), &lookup);
            let differs = eval(&format!("{left} != {right}"), &lookup);
            assert_eq!(forward, backward, "{left} == {right} is symmetric");
            assert_eq!(differs, json!(!forward.as_bool().expect("boolean")), "{left} != {right} complements");
        }
    }
    // Distinct corpus entries differ except the pairs written out here.
    let equal_pairs = [("missing", "explicit_null"), ("0", "negative_zero"), ("two_pow_53", "two_pow_53_float")];
    for (index, left) in corpus.iter().enumerate() {
        for (offset, right) in corpus[index + 1..].iter().enumerate() {
            let expected = equal_pairs.contains(&(left, right));
            let actual = eval(&format!("{left} == {right}"), &lookup) == json!(true);
            assert_eq!(actual, expected, "{left} == {right} ({:?} vs {:?})", samples[index], samples[index + 1 + offset]);
        }
    }
}
