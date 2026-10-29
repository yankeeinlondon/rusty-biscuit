//! The coercion engine's function-call rules through normal dispatch
//! (`evaluate(parse(..))`), with the functions the spec names
//! "test-registered" registered only here (spec criteria 16, 17, 19–22, and
//! 24–27). Each handler records what it received, and its entry counter stays
//! at 0 on every rejection. The binder-level versions of these tables are in
//! `function_call_binding.rs`; criteria 14, 15, and 18 through dispatch are in
//! `pure_function_contracts.rs`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use darkmatter::markdown::coercion::{FailureReason, ValueKind};
use darkmatter::markdown::compose::expression::call_binding::{
    ArgumentProblem, CallBindingError, CallErrorKind, FunctionRegistration,
};
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse};
use serde_json::{Value, json};

/// Variables the expressions read, as frontmatter would supply them.
#[derive(Default)]
struct Values(HashMap<String, Value>);

impl EvaluationLookup for Values {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }
}

/// A test-registered function whose handler records each call's arguments
/// and returns `returns`.
struct Recorded {
    name: &'static str,
    received: Arc<Mutex<Vec<Vec<Value>>>>,
    _guard: TestFunctionGuard,
}

impl Recorded {
    fn new(name: &'static str, signature: &str, returns: Value) -> Self {
        let registration = FunctionRegistration::builder(name, signature).build().expect("the test signature compiles");
        let received = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&received);
        let guard = hooks::register(
            registration,
            Arc::new(move |call| {
                let args = (0..call.args().len()).map(|index| call.args().get(index).cloned().expect("bound")).collect();
                sink.lock().expect("recorder").push(args);
                Ok(returns.clone())
            }),
        );
        Self { name, received, _guard: guard }
    }

    /// One parameter `parameter: ty`, returning a string.
    fn one(name: &'static str, parameter: &str, ty: &str) -> Self {
        Self::new(name, &format!("function(parameters({parameter}: {ty}); returns(string))"), json!("done"))
    }

    /// The arguments the handler received for `expression`, which must bind
    /// and enter the handler exactly once.
    fn received(&self, expression: &str, values: &Values) -> Vec<Value> {
        self.received.lock().expect("recorder").clear();
        hooks::reset_handler_entries();
        run(expression, values).unwrap_or_else(|error| panic!("{expression}: {error}"));
        assert_eq!(hooks::handler_entries(self.name), 1, "{expression}: handler entries");
        self.received.lock().expect("recorder").pop().expect("the handler recorded its call")
    }

    /// The binding error for `expression`, which must never enter the
    /// handler.
    fn rejected(&self, expression: &str, values: &Values) -> CallBindingError {
        rejected(self.name, expression, values)
    }
}

fn run(expression: &str, values: &Values) -> Result<Value, ExpressionError> {
    evaluate(&parse(expression).unwrap_or_else(|error| panic!("{expression}: {error}")), values)
}

fn rejected(function: &str, expression: &str, values: &Values) -> CallBindingError {
    hooks::reset_handler_entries();
    let error = match run(expression, values) {
        Err(ExpressionError::Call(error)) => *error,
        other => panic!("{expression}: expected a binding error, got {other:?}"),
    };
    assert_eq!(hooks::handler_entries(function), 0, "{expression}: a rejected call never enters the handler");
    error
}

fn reasons(error: &CallBindingError) -> Vec<FailureReason> {
    error
        .argument_errors()
        .flat_map(|argument| match &argument.problem {
            ArgumentProblem::Conversion(conversion) => {
                conversion.failures.iter().map(|failure| failure.reason.clone()).collect()
            }
            _ => Vec::new(),
        })
        .collect()
}

fn no_values() -> Values {
    Values::default()
}

fn values(pairs: &[(&str, Value)]) -> Values {
    Values(pairs.iter().map(|(name, value)| (name.to_string(), value.clone())).collect())
}

/// Criterion 16: the built-in `min` reports the function, parameter, expected
/// type, received value and kind, and reason, and is never entered.
#[test]
fn min_pear_type_error_names_function_parameter_type_value_reason() {
    let error = rejected("min", r#"min("pear", 5)"#, &no_values());

    assert_eq!(error.kind(), CallErrorKind::Type);
    assert_eq!(error.function(), "min");
    let arguments: Vec<_> = error.argument_errors().collect();
    assert_eq!(arguments.len(), 1, "{error}");
    let argument = arguments[0];
    assert_eq!((argument.parameter.as_str(), argument.index), ("a", 0));
    assert_eq!(argument.expected, "number");
    assert_eq!(argument.received, json!("pear"));
    assert_eq!(argument.received_kind(), ValueKind::Text);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::MalformedText { .. }]), "{error}");
    let message = ExpressionError::Call(Box::new(error)).to_string();
    for part in ["min", "a (argument 1)", "expected number", "text \"pear\""] {
        assert!(message.contains(part), "{part:?} missing from:\n{message}");
    }
}

/// Criterion 17: a number converts to text and then meets the length
/// constraints; text or a number that breaks them is rejected, never
/// truncated.
#[test]
fn three_letter_acronym_constraints() {
    let tla = Recorded::new(
        "three_letter_acronym",
        "function(parameters(tla: string(min(3); max(3))); returns(string))",
        json!("TLA"),
    );
    assert_eq!(tla.received("three_letter_acronym(123)", &no_values()), [json!("123")]);
    for expression in ["three_letter_acronym(1234)", r#"three_letter_acronym("ABCD")"#] {
        let error = tla.rejected(expression, &no_values());
        assert!(matches!(reasons(&error).as_slice(), [FailureReason::ConstraintNotMet(_)]), "{expression}: {error}");
    }
}

/// Criterion 17: `"2.5"` converts to `2.5`, which fails `integer`; it is
/// never rounded.
#[test]
fn integer_constraint_rejects_two_point_five() {
    let f = Recorded::new("f", "function(parameters(n: number(integer)); returns(number))", json!(0));
    let error = f.rejected(r#"f("2.5")"#, &no_values());
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::ConstraintNotMet(_)]), "{error}");
    assert_eq!(f.received(r#"f("4")"#, &no_values()), [json!(4)]);
}

/// Criterion 19: boolean → number, null → anything, and wrapping a value
/// into a list are rejected before any handler runs.
#[test]
fn boolean_null_and_text_list_are_rejected() {
    let error = rejected("min", "min(true, 5)", &no_values());
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::NoConversion]), "{error}");
    let error = rejected("min", "min(null, 5)", &no_values());
    let argument = error.argument_errors().next().expect("one argument error");
    assert_eq!((argument.parameter.as_str(), &argument.problem), ("a", &ArgumentProblem::NullNotAdmitted));

    let sum = Recorded::new("sum", "function(parameters(values: number[]); returns(number))", json!(0));
    let error = sum.rejected(r#"sum("1")"#, &no_values());
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::NoConversion]), "{error}");
}

/// Registers `name(parameter: first | second)` and the reversed union in
/// turn, handing each to `check`.
fn both_orders(name: &'static str, parameter: &str, first: &str, second: &str, check: impl Fn(&Recorded)) {
    for (a, b) in [(first, second), (second, first)] {
        let function = Recorded::one(name, parameter, &format!("{a} | {b}"));
        check(&function);
    }
}

/// Criterion 20: every row of the spec's union table, with the options
/// written in either order.
#[test]
fn union_rule_table_both_orders() {
    let none = no_values();
    both_orders("describe", "val", "number", "boolean", |describe| {
        assert_eq!(describe.received(r#"describe("1")"#, &none), [json!(1)]);
    });
    both_orders("g", "val", "number(min(5))", "boolean", |g| {
        assert_eq!(g.received(r#"g("1")"#, &none), [json!(true)]);
    });
    both_orders("s", "val", "string", "string[]", |s| {
        assert_eq!(s.received("s(2026)", &none), [json!("2026")]);
    });
    both_orders("k", "val", "{ id: number }", "{ id: boolean }", |k| {
        assert_eq!(k.received(r#"k({id: "1"})"#, &none), [json!({"id": 1})]);
    });
    both_orders("m", "val", "{ a: number, b: boolean }", "{ a: boolean, b: number }", |m| {
        let error = m.rejected(r#"m({a: "1", b: "1"})"#, &none);
        match reasons(&error).as_slice() {
            [FailureReason::AmbiguousUnion { readings }] => {
                assert_eq!(readings.len(), 2, "{error}");
                assert!(readings.contains(&json!({"a": 1, "b": true})), "{error}");
                assert!(readings.contains(&json!({"a": true, "b": 1})), "{error}");
            }
            other => panic!("expected an ambiguous union, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.contains(r#"{"a":1,"b":true}"#) && message.contains(r#"{"a":true,"b":1}"#), "{message}");
    });
    both_orders("c", "config", "json", "yaml", |c| {
        assert_eq!(
            c.received("c({port: 8080, debug: true})", &none),
            [json!(r#"{"port":8080,"debug":true}"#)]
        );
        assert_eq!(c.received(r#"c("port: 8080")"#, &none), [json!("port: 8080")]);
    });
}

/// Criterion 21: whole numbers must be represented exactly; decimals take
/// the nearest `f64`.
#[test]
fn echo_exactness_table() {
    let echo = Recorded::new("echo", "function(parameters(val: number); returns(number))", json!(0));
    let none = no_values();
    for (text, expected) in [
        ("9007199254740992", json!(9_007_199_254_740_992_u64)),
        ("1e3", json!(1000)),
        ("0.1", json!(0.1)),
    ] {
        assert_eq!(echo.received(&format!("echo({text:?})"), &none), [expected], "{text}");
    }
    for text in ["9007199254740993", "18446744073709551615", "99999999999999999999", "1e400", "1e-400"] {
        let error = echo.rejected(&format!("echo({text:?})"), &none);
        assert!(matches!(reasons(&error).as_slice(), [FailureReason::NumberNotAdmitted(_)]), "{text}: {error}");
        let message = error.to_string();
        assert!(
            message.contains("cannot be represented exactly") || message.contains("not finite"),
            "{text}: {message}"
        );
    }
}

/// Criterion 21: the expression literal `9007199254740993` gets the same
/// exactness check, so the handler is never entered.
#[test]
fn literal_beyond_safe_integer_is_rejected() {
    let echo = Recorded::new("echo", "function(parameters(val: number); returns(number))", json!(0));
    hooks::reset_handler_entries();
    let message = match parse("echo(9007199254740993)") {
        Err(error) => error.to_string(),
        Ok(expression) => match evaluate(&expression, &no_values()) {
            Err(error) => error.to_string(),
            Ok(value) => panic!("the literal was admitted as {value}"),
        },
    };
    assert!(message.contains("cannot be represented exactly"), "{message}");
    assert_eq!(hooks::handler_entries(echo.name), 0);
    assert_eq!(echo.received("echo(9007199254740992)", &no_values()), [json!(9_007_199_254_740_992_u64)]);
}

/// Criterion 22: with `low` absent and `high` holding `"pear"`, one type
/// error reports both of `min`'s parameters.
#[test]
fn one_error_reports_both_parameters() {
    let error = rejected("min", "min(low, high)", &values(&[("high", json!("pear"))]));

    assert_eq!(error.kind(), CallErrorKind::Type);
    let named: Vec<&str> = error.argument_errors().map(|argument| argument.parameter.as_str()).collect();
    assert_eq!(named, ["a", "b"], "{error}");
    let message = error.to_string();
    assert!(message.contains("a (argument 1)") && message.contains("b (argument 2)"), "{message}");
}

/// Criterion 24: every row of the spec's Containers table; the caller's list
/// is unchanged after a converting call.
#[test]
fn containers_table_person_and_sum() {
    let person = Recorded::new(
        "person",
        "function(parameters(p: tuple([string, number, boolean?])); returns(string))",
        json!("Ada"),
    );
    let full = json!(["Ada", "36", "yes"]);
    let caller = values(&[("p", full.clone())]);
    assert_eq!(person.received("person(p)", &caller), [json!(["Ada", 36, true])]);
    assert_eq!(caller.0["p"], full, "the caller's list is unchanged");
    assert_eq!(person.received(r#"person(["Ada", "36"])"#, &no_values()), [json!(["Ada", 36])]);
    let error = person.rejected(r#"person(["Ada"])"#, &no_values());
    assert_eq!(error.argument_errors().next().expect("one argument error").paths(), ["p[1]"], "{error}");
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::MissingRequired]), "{error}");

    let sum = Recorded::new("sum", "function(parameters(values: number[]); returns(number))", json!(0));
    let error = sum.rejected(r#"sum(["1", "2", "x"])"#, &no_values());
    assert_eq!(error.argument_errors().next().expect("one argument error").paths(), ["values[2]"], "{error}");
    assert!(error.to_string().contains("values[2]"), "{error}");
    let error = sum.rejected(r#"sum("1")"#, &no_values());
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::NoConversion]), "{error}");
}

/// Criterion 25: a native value serializes into `json`; text is validated
/// and kept.
#[test]
fn json_parameter_serializes_native_and_keeps_text() {
    let h = Recorded::one("h", "doc", "json");
    assert_eq!(h.received("h({a: 1})", &no_values()), [json!(r#"{"a":1}"#)]);
    assert_eq!(h.received(r#"h("{\"a\":1}")"#, &no_values()), [json!(r#"{"a":1}"#)]);
    h.rejected(r#"h("{a")"#, &no_values());
}

/// Criterion 26: `numberlike` accepts what `number` would and delivers it as
/// sent.
#[test]
fn numberlike_parameter_delivers_original() {
    let n = Recorded::one("n", "val", "numberlike");
    for (expression, sent) in [("n(5)", json!(5)), (r#"n("5")"#, json!("5")), (r#"n("02134")"#, json!("02134")), (r#"n(" 4 ")"#, json!(" 4 "))] {
        assert_eq!(n.received(expression, &no_values()), [sent], "{expression}");
    }
    for expression in [r#"n("pear")"#, "n(true)", "n(null)"] {
        n.rejected(expression, &no_values());
    }
}

/// Criterion 27: `boolish` accepts what `boolean` would and delivers it as
/// sent.
#[test]
fn boolish_parameter_delivers_original() {
    let b = Recorded::one("b", "val", "boolish");
    for (expression, sent) in [("b(true)", json!(true)), (r#"b("Yes")"#, json!("Yes"))] {
        assert_eq!(b.received(expression, &no_values()), [sent], "{expression}");
    }
    for expression in ["b(1)", r#"b("maybe")"#] {
        b.rejected(expression, &no_values());
    }
}
