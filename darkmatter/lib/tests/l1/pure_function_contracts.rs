//! Pure expression functions through normal dispatch (`evaluate(parse(...))`):
//! the approved contracts of the one-pass migration (spec criteria 12, 14,
//! 15, 18, 19, and the pure parts of 43–45). Each case also counts handler
//! entries: a call that does not bind never enters its handler, and an
//! accepted call enters it exactly once.

use darkmatter::markdown::compose::expression::call_binding::CallErrorKind;
use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
use darkmatter::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse};
use serde_json::{Value, json};

struct NoValues;

impl EvaluationLookup for NoValues {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }
}

/// Evaluates `expression` with fresh counters and returns the result and how
/// often `function`'s handler was entered.
fn run(function: &str, expression: &str) -> (Result<Value, ExpressionError>, usize) {
    hooks::reset_handler_entries();
    let result = evaluate(&parse(expression).expect("the expression parses"), &NoValues);
    (result, hooks::handler_entries(function))
}

/// Asserts `expression` returns `expected` after entering `function` once.
fn accepts(function: &str, expression: &str, expected: Value) {
    let (result, entries) = run(function, expression);
    assert_eq!(result.unwrap_or_else(|error| panic!("{expression}: {error}")), expected, "{expression}");
    assert_eq!(entries, 1, "{expression}: handler entries");
}

/// Asserts `expression` fails to bind with `kind` and never enters `function`.
fn rejects(function: &str, expression: &str, kind: CallErrorKind) {
    let (result, entries) = run(function, expression);
    match result {
        Err(ExpressionError::Call(error)) => assert_eq!(error.kind(), kind, "{expression}: {error}"),
        other => panic!("{expression}: expected a {kind:?} binding error, got {other:?}"),
    }
    assert_eq!(entries, 0, "{expression}: handler entries");
}

/// Criterion 12: `number()` follows the charter contract.
#[test]
fn number_converts_numberlike_and_recovers_through_its_fallback() {
    accepts("number", r#"number("4")"#, json!(4));
    accepts("number", r#"number(" 4 ")"#, json!(4));
    for subject in [r#""pear""#, "null", "true", "[]", r#""9007199254740993""#] {
        accepts("number", &format!("number({subject}, 7)"), json!(7));
    }
    rejects("number", r#"number("pear")"#, CallErrorKind::Type);
    rejects("number", r#"number("4", "x")"#, CallErrorKind::Type);
    rejects("number", "number()", CallErrorKind::Arity);
}

/// Criterion 12: `round()` rounds half away from zero, fallback included.
#[test]
fn round_rounds_half_away_from_zero_and_recovers_through_its_fallback() {
    accepts("round", r#"round("3.7")"#, json!(4));
    accepts("round", "round(-2.5)", json!(-3));
    accepts("round", r#"round("pear", 2.7)"#, json!(3));
    rejects("round", r#"round("pear")"#, CallErrorKind::Type);
}

/// Criteria 14 and 19: numeric text converts for `min`; booleans and null do
/// not.
#[test]
fn min_converts_numeric_text_and_rejects_booleans_and_null() {
    accepts("min", r#"min("4", 5)"#, json!(4));
    rejects("min", "min(true, 5)", CallErrorKind::Type);
    rejects("min", "min(null, 5)", CallErrorKind::Type);
}

/// Criterion 15: a number converts to text for a text parameter.
#[test]
fn upper_converts_a_number_to_text() {
    accepts("upper", "upper(2026)", json!("2026"));
}

/// Criterion 18: an `unknown` parameter keeps the original value.
#[test]
fn is_number_inspects_the_original_value() {
    accepts("is_number", r#"is_number("4")"#, json!(false));
    accepts("is_number", "is_number(4)", json!(true));
}

/// Criterion 43 (pure parts): text transforms pass null through, predicates
/// answer `false`, and arithmetic rejects null.
#[test]
fn the_ledger_null_rule_holds_for_pure_functions() {
    accepts("upper", "upper(null)", Value::Null);
    accepts("trim", "trim(null)", Value::Null);
    accepts("has_key", r#"has_key(null, "a")"#, json!(false));
    rejects("min", "min(null, 5)", CallErrorKind::Type);
}

/// Criterion 44 (pure parts): `length` and `contains` follow charter M10/M11.
#[test]
fn length_and_contains_follow_the_charter() {
    rejects("length", "length(true)", CallErrorKind::Type);
    accepts("length", r#"length("true")"#, json!(4));
    accepts("length", "length(2.5)", json!(3));
    accepts("length", "length({a: 1})", json!(1));
    accepts("contains", r#"contains([4], "4")"#, json!(true));
    accepts("contains", r#"contains({a: "x"}, "a")"#, json!(false));
}

/// Criterion 45: map functions keep the written key order.
#[test]
fn keys_and_values_keep_insertion_order() {
    accepts("keys", "keys({b: 1, a: 2})", json!(["b", "a"]));
    accepts("values", "values({b: 1, a: 2})", json!([1, 2]));
}
