//! The call binder through its public API: test-registered signatures written
//! in the grammar, bound and then handed to a counting handler only on
//! success (spec criteria 12 and 14–27, plus source exclusions, presence,
//! rest parameters, laziness, return contracts, overloads, and the grammar
//! translator).

use std::cell::Cell;

use darkmatter::markdown::coercion::{FailureReason, ValueKind, bind};
use darkmatter::markdown::compose::expression::call_binding::{
    ArgumentProblem, BoundCall, CallBindingError, CallErrorKind, EvaluationMode, FunctionRegistration, NoNamedTypes,
    RegistrationError, SignatureProblem, TranslateError, property_shape_from_def, shape_from_property_def,
};
use darkmatter::markdown::schemas::simplified::grammar::parse_type_definition;
use darkmatter::markdown::schemas::simplified::{FunctionType, PropertyDef, TypeExpr};
use serde_json::{Value, json};

/// A registered function whose handler only counts its invocations.
struct Function {
    registration: FunctionRegistration,
    calls: Cell<usize>,
}

impl Function {
    fn new(name: &str, signatures: &str) -> Self {
        Self::from(FunctionRegistration::builder(name, signatures).build().expect("registration builds"))
    }

    fn from(registration: FunctionRegistration) -> Self {
        Self {
            registration,
            calls: Cell::new(0),
        }
    }

    /// Binds, then invokes the handler only when binding succeeded. The
    /// caller's values must come back unchanged either way.
    fn call(&self, args: &[Value]) -> Result<BoundCall<'static>, CallBindingError> {
        let before = args.to_vec();
        let result = self.registration.bind_call(args).map(|call| {
            self.calls.set(self.calls.get() + 1);
            call.into_owned()
        });
        assert_eq!(args, before.as_slice(), "binding never mutates the caller's values");
        result
    }

    fn calls(&self) -> usize {
        self.calls.get()
    }

    /// The handler's received arguments for a successful call.
    fn received(&self, args: &[Value]) -> Vec<Value> {
        let call = self.call(args).unwrap_or_else(|error| panic!("expected a bound call, got: {error}"));
        (0..call.args().len()).map(|index| call.args().get(index).cloned().expect("bound")).collect()
    }

    /// A rejected call: returns the error, asserting the handler never ran.
    fn rejected(&self, args: &[Value]) -> CallBindingError {
        let before = self.calls();
        let error = match self.call(args) {
            Ok(call) => panic!("expected an error, bound {:?}", call.args()),
            Err(error) => error,
        };
        assert_eq!(self.calls(), before, "a rejected call never reaches the handler");
        error
    }
}

fn both_orders(name: &str, parameter: &str, first: &str, second: &str, returns: &str) -> [Function; 2] {
    [
        Function::new(name, &format!("function(parameters({parameter}: {first} | {second}); returns({returns}))")),
        Function::new(name, &format!("function(parameters({parameter}: {second} | {first}); returns({returns}))")),
    ]
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

fn min() -> Function {
    Function::new("min", "function(parameters(a: number, b: number); returns(number))")
}

// ── criterion 14–16 ─────────────────────────────────────────────────────────

#[test]
fn text_converts_to_a_number_and_the_handler_runs_once() {
    let min = min();
    assert_eq!(min.received(&[json!("4"), json!(5)]), [json!(4), json!(5)]);
    assert_eq!(min.calls(), 1);
}

#[test]
fn number_converts_to_text() {
    let upper = Function::new("upper", "function(parameters(value: string | null); returns(string | null))");
    assert_eq!(upper.received(&[json!(2026)]), [json!("2026")]);
}

#[test]
fn type_error_names_function_parameter_type_value_and_reason() {
    let min = min();
    let error = min.rejected(&[json!("pear"), json!(5)]);
    assert_eq!(error.kind(), CallErrorKind::Type);
    assert_eq!(error.function(), "min");
    let arguments: Vec<_> = error.argument_errors().collect();
    assert_eq!(arguments.len(), 1);
    let argument = arguments[0];
    assert_eq!((argument.parameter.as_str(), argument.index), ("a", 0));
    assert_eq!(argument.expected, "number");
    assert_eq!(argument.received, json!("pear"));
    assert_eq!(argument.received_kind(), ValueKind::Text);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::MalformedText { .. }]));
    let message = error.to_string();
    for part in ["min", "min(a: number, b: number): number", "a (argument 1)", "expected number", "text \"pear\""] {
        assert!(message.contains(part), "{part:?} missing from:\n{message}");
    }
    assert_eq!(min.calls(), 0);
}

// ── criterion 17–19 ─────────────────────────────────────────────────────────

#[test]
fn constraints_are_enforced_never_repaired() {
    let tla = Function::new(
        "three_letter_acronym",
        "function(parameters(tla: string(min(3); max(3))); returns(string))",
    );
    assert_eq!(tla.received(&[json!(123)]), [json!("123")]);
    for value in [json!(1234), json!("ABCD")] {
        let error = tla.rejected(&[value]);
        assert!(matches!(reasons(&error).as_slice(), [FailureReason::ConstraintNotMet(_)]), "{error}");
    }
    let f = Function::new("f", "function(parameters(n: number(integer)); returns(number))");
    let error = f.rejected(&[json!("2.5")]);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::ConstraintNotMet(_)]), "{error}");
    assert_eq!((tla.calls(), f.calls()), (1, 0));
}

#[test]
fn any_keeps_the_original_value() {
    let is_number = Function::new("is_number", "function(parameters(val: unknown | null); returns(boolean))");
    let call = is_number.call(&[json!("4")]).expect("binds");
    assert!(call.is_exact());
    assert_eq!(call.args().get(0), Some(&json!("4")));
    assert_eq!(is_number.received(&[json!(4)]), [json!(4)]);
    assert_eq!(is_number.received(&[Value::Null]), [Value::Null]);
}

#[test]
fn rejected_conversions_are_rejected() {
    let min = min();
    let error = min.rejected(&[json!(true), json!(5)]);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::NoConversion]), "{error}");
    let error = min.rejected(&[Value::Null, json!(5)]);
    let null = error.argument_errors().next().expect("one argument error");
    assert_eq!((null.parameter.as_str(), &null.problem), ("a", &ArgumentProblem::NullNotAdmitted));
    let sum = Function::new("sum", "function(parameters(values: number[]); returns(number))");
    let error = sum.rejected(&[json!("1")]);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::NoConversion]), "{error}");
}

// ── criterion 20 ────────────────────────────────────────────────────────────

#[test]
fn union_number_beats_boolean() {
    for describe in both_orders("describe", "val", "number", "boolean", "string") {
        assert_eq!(describe.received(&[json!("1")]), [json!(1)]);
    }
}

#[test]
fn union_constraint_failure_leaves_one_reachable_option() {
    for g in both_orders("g", "val", "number(min(5))", "boolean", "string") {
        assert_eq!(g.received(&[json!("1")]), [json!(true)]);
    }
}

#[test]
fn union_only_text_is_reachable_from_a_number() {
    for s in both_orders("s", "val", "string", "string[]", "string") {
        assert_eq!(s.received(&[json!(2026)]), [json!("2026")]);
    }
}

#[test]
fn union_of_objects_ties_break_field_by_field() {
    for k in both_orders("k", "val", "{id: number}", "{id: boolean}", "string") {
        assert_eq!(k.received(&[json!({"id": "1"})]), [json!({"id": 1})]);
    }
}

#[test]
fn union_of_objects_with_crossed_ties_is_ambiguous() {
    for m in both_orders("m", "val", "{a: number, b: boolean}", "{a: boolean, b: number}", "string") {
        let error = m.rejected(&[json!({"a": "1", "b": "1"})]);
        match reasons(&error).as_slice() {
            [FailureReason::AmbiguousUnion { readings }] => {
                assert_eq!(readings.len(), 2);
                assert!(readings.contains(&json!({"a": 1, "b": true})));
                assert!(readings.contains(&json!({"a": true, "b": 1})));
            }
            other => panic!("expected an ambiguous union, got {other:?}"),
        }
        let message = error.to_string();
        assert!(message.contains(r#"{"a":1,"b":true}"#) && message.contains(r#"{"a":true,"b":1}"#), "{message}");
        assert_eq!(m.calls(), 0);
    }
}

#[test]
fn union_of_json_and_yaml_prefers_json_and_keeps_text() {
    for c in both_orders("c", "config", "json", "yaml", "string") {
        assert_eq!(c.received(&[json!({"port": 8080, "debug": true})]), [json!(r#"{"port":8080,"debug":true}"#)]);
        let call = c.call(&[json!("port: 8080")]).expect("binds");
        assert!(call.is_exact());
        assert_eq!(call.args().get(0), Some(&json!("port: 8080")));
    }
}

// ── criterion 21 ────────────────────────────────────────────────────────────

#[test]
fn exactness_check_admits_representable_numbers() {
    let echo = Function::new("echo", "function(parameters(val: number); returns(number))");
    assert_eq!(echo.received(&[json!("9007199254740992")]), [json!(9_007_199_254_740_992_u64)]);
    assert_eq!(echo.received(&[json!("1e3")]), [json!(1000)]);
    assert_eq!(echo.received(&[json!("0.1")]), [json!(0.1)]);
}

#[test]
fn exactness_check_rejects_unrepresentable_numbers() {
    let echo = Function::new("echo", "function(parameters(val: number); returns(number))");
    for text in ["9007199254740993", "18446744073709551615", "99999999999999999999", "1e400", "1e-400"] {
        let error = echo.rejected(&[json!(text)]);
        assert!(
            matches!(reasons(&error).as_slice(), [FailureReason::NumberNotAdmitted(_)]),
            "{text}: {error}"
        );
        let message = error.to_string();
        assert!(
            message.contains("cannot be represented exactly") || message.contains("not finite"),
            "{text}: {message}"
        );
    }
    assert_eq!(echo.calls(), 0);
}

// ── criterion 22 ────────────────────────────────────────────────────────────

#[test]
fn every_argument_is_checked() {
    let min = min();
    let error = min.rejected(&[Value::Null, json!("pear")]);
    let named: Vec<&str> = error.argument_errors().map(|argument| argument.parameter.as_str()).collect();
    assert_eq!(named, ["a", "b"]);
    let message = error.to_string();
    assert!(message.contains("a (argument 1)") && message.contains("b (argument 2)"), "{message}");
}

// ── criterion 23 ────────────────────────────────────────────────────────────

const PR_COUNT: &str = "function(parameters(count: number(integer; min(1))); returns(string[]))";
const PR_QUERY: &str = "function(parameters(query: object); returns(string[]))";

fn pr_lists() -> [Function; 2] {
    [
        Function::new("pr_list", &format!("{PR_COUNT} | {PR_QUERY}")),
        Function::new("pr_list", &format!("{PR_QUERY} | {PR_COUNT}")),
    ]
}

#[test]
fn overloads_select_count_for_numeric_text_and_numbers() {
    for pr_list in pr_lists() {
        for value in [json!("3"), json!(3)] {
            let call = pr_list.call(std::slice::from_ref(&value)).expect("binds");
            let selected = &pr_list.registration.overloads()[call.overload()];
            assert_eq!(selected.parameters()[0].name(), "count", "{value}");
            assert_eq!(call.args().get(0), Some(&json!(3)));
        }
        assert_eq!(pr_list.calls(), 2);
    }
}

#[test]
fn overload_failure_lists_every_signature_and_reason() {
    for pr_list in pr_lists() {
        let error = pr_list.rejected(&[json!("0")]);
        let CallBindingError::Type { candidates, .. } = &error else {
            panic!("expected a type error, got {error}");
        };
        assert_eq!(candidates.len(), 2);
        for candidate in candidates {
            assert!(matches!(&candidate.problem, SignatureProblem::Arguments(errors) if errors.len() == 1));
        }
        let message = error.to_string();
        for part in [
            "pr_list: no signature accepts these arguments",
            "pr_list(count: number(integer;min(1))): string[]",
            "pr_list(query: object): string[]",
            "count (argument 1)",
            "converted to 0; fails min(1)",
            "query (argument 1)",
        ] {
            assert!(message.contains(part), "{part:?} missing from:\n{message}");
        }
        let error = pr_list.rejected(&[json!("2.5")]);
        assert_eq!(error.kind(), CallErrorKind::Type);
        assert!(error.to_string().contains("integer"), "{error}");
    }
}

#[test]
fn a_received_number_reads_as_interpolation_presents_it() {
    let pr_list = &pr_lists()[0];
    // An expression literal `0` arrives as the float `0.0`.
    for (value, shown) in [(json!(0.0), "received number 0;"), (json!(2.5), "received number 2.5;"), (json!(0), "received number 0;")] {
        let message = pr_list.rejected(std::slice::from_ref(&value)).to_string();
        assert!(message.contains(shown), "{shown:?} missing from:\n{message}");
        assert!(!message.contains("0.0"), "float spelling in:\n{message}");
    }
    let message = pr_list.rejected(&[json!(u64::MAX)]).to_string();
    assert!(message.contains("received number 18446744073709551615;"), "{message}");
}

// ── criterion 24 ────────────────────────────────────────────────────────────

#[test]
fn tuples_follow_the_declared_structure() {
    let person = Function::new("person", "function(parameters(p: tuple([string, number, boolean?])); returns(string))");
    assert_eq!(person.received(&[json!(["Ada", "36", "yes"])]), [json!(["Ada", 36, true])]);
    assert_eq!(person.received(&[json!(["Ada", "36"])]), [json!(["Ada", 36])]);
    let error = person.rejected(&[json!(["Ada"])]);
    let argument = error.argument_errors().next().expect("one argument error");
    assert_eq!(argument.paths(), ["p[1]"]);
    assert!(matches!(reasons(&error).as_slice(), [FailureReason::MissingRequired]));
    assert!(error.to_string().contains("p[1] (argument 1)"), "{error}");
}

#[test]
fn arrays_follow_the_declared_structure() {
    let sum = Function::new("sum", "function(parameters(values: number[]); returns(number))");
    let error = sum.rejected(&[json!(["1", "2", "x"])]);
    assert_eq!(error.argument_errors().next().expect("one").paths(), ["values[2]"]);
    assert!(error.to_string().contains("values[2] (argument 1)"), "{error}");
    sum.rejected(&[json!("1")]);
    assert_eq!(sum.received(&[json!(["1", 2])]), [json!([1, 2])]);
}

// ── criterion 25–27 ─────────────────────────────────────────────────────────

#[test]
fn native_values_serialize_into_json() {
    let h = Function::new("h", "function(parameters(doc: json); returns(string))");
    assert_eq!(h.received(&[json!({"a": 1})]), [json!(r#"{"a":1}"#)]);
    assert_eq!(h.received(&[json!(r#"{"a":1}"#)]), [json!(r#"{"a":1}"#)]);
    h.rejected(&[json!("{a")]);
}

#[test]
fn numberlike_delivers_the_value_as_sent() {
    let n = Function::new("n", "function(parameters(val: numberlike); returns(string))");
    for value in [json!(5), json!("5"), json!("02134"), json!(" 4 ")] {
        assert_eq!(n.received(std::slice::from_ref(&value)), [value]);
    }
    for value in [json!("pear"), json!(true), Value::Null] {
        n.rejected(&[value]);
    }
    assert_eq!(n.calls(), 4);
}

#[test]
fn boolish_delivers_the_value_as_sent() {
    let b = Function::new("b", "function(parameters(val: boolish); returns(string))");
    for value in [json!(true), json!("Yes")] {
        assert_eq!(b.received(std::slice::from_ref(&value)), [value]);
    }
    for value in [json!(1), json!("maybe")] {
        b.rejected(&[value]);
    }
    assert_eq!(b.calls(), 2);
}

// ── criterion 12: numeric recovery at the binding level ────────────────────

fn recovering(name: &str) -> Function {
    Function::from(
        FunctionRegistration::builder(name, "function(parameters(x: numberlike, fallback?: number); returns(number))")
            .numeric_recovery("x", "fallback")
            .build()
            .expect("registration builds"),
    )
}

#[test]
fn recovery_selects_a_valid_fallback_when_the_subject_fails() {
    let number = recovering("number");
    for subject in [json!("pear"), Value::Null, json!(true), json!([]), json!("9007199254740993")] {
        let call = number.call(&[subject.clone(), json!(7)]).expect("recovers");
        let recovery = call.args().recovery().expect("recovery selected");
        assert_eq!((recovery.subject, recovery.fallback), (0, 1));
        assert_eq!(call.args().selected(0), Some(&json!(7)), "{subject}");
        assert_eq!(call.args().get(0), None, "a recovered subject has no bound value");
    }
    let round = recovering("round");
    let call = round.call(&[json!("pear"), json!(2.7)]).expect("recovers");
    assert_eq!(call.args().selected(0), Some(&json!(2.7)));
}

#[test]
fn recovery_without_a_fallback_keeps_the_type_error() {
    let number = recovering("number");
    let error = number.rejected(&[json!("pear")]);
    assert_eq!(error.kind(), CallErrorKind::Type);
    assert_eq!(error.argument_errors().next().expect("one").parameter, "x");
    let round = recovering("round");
    round.rejected(&[json!("pear")]);
}

#[test]
fn a_supplied_fallback_is_always_validated() {
    let number = recovering("number");
    let error = number.rejected(&[json!("4"), json!("x")]);
    let named: Vec<&str> = error.argument_errors().map(|argument| argument.parameter.as_str()).collect();
    assert_eq!(named, ["fallback"]);
    let error = number.rejected(&[json!("pear"), json!("x")]);
    let named: Vec<&str> = error.argument_errors().map(|argument| argument.parameter.as_str()).collect();
    assert_eq!(named, ["x", "fallback"]);
}

#[test]
fn recovery_never_covers_a_missing_subject() {
    let number = recovering("number");
    assert_eq!(number.rejected(&[]).kind(), CallErrorKind::Arity);
}

#[test]
fn a_bound_subject_is_used_unchanged() {
    let number = recovering("number");
    let call = number.call(&[json!("4"), json!(7)]).expect("binds");
    assert!(call.args().recovery().is_none());
    assert_eq!(call.args().selected(0), Some(&json!("4")));
    assert_eq!(number.received(&[json!(" 4 ")]), [json!(" 4 ")]);
}

#[test]
fn recovery_registration_is_validated() {
    let wrong_fallback = FunctionRegistration::builder(
        "number",
        "function(parameters(x: numberlike, fallback?: string); returns(number))",
    )
    .numeric_recovery("x", "fallback")
    .build();
    assert!(matches!(wrong_fallback, Err(RegistrationError::InvalidRecovery { .. })));
    let unknown = FunctionRegistration::builder("number", "function(parameters(x: numberlike); returns(number))")
        .numeric_recovery("x", "fallback")
        .build();
    assert!(matches!(unknown, Err(RegistrationError::InvalidRecovery { .. })));
}

// ── source exclusions ───────────────────────────────────────────────────────

fn length() -> Function {
    Function::from(
        FunctionRegistration::builder("length", "function(parameters(val: string | any[] | object); returns(number))")
            .reject_sources("val", &[ValueKind::Boolean])
            .build()
            .expect("registration builds"),
    )
}

#[test]
fn source_exclusion_rejects_before_conversion() {
    let length = length();
    let error = length.rejected(&[json!(true)]);
    let argument = error.argument_errors().next().expect("one");
    assert_eq!(argument.problem, ArgumentProblem::SourceExcluded);
    assert!(error.to_string().contains("val (argument 1)"), "{error}");
    assert_eq!(length.received(&[json!(2.5)]), [json!("2.5")]);
    assert_eq!(length.received(&[json!([1, 2])]), [json!([1, 2])]);
}

#[test]
fn source_exclusion_names_a_declared_parameter() {
    let unknown = FunctionRegistration::builder("length", "function(parameters(val: string); returns(number))")
        .reject_sources("value", &[ValueKind::Boolean])
        .build();
    assert!(matches!(unknown, Err(RegistrationError::UnknownParameter { .. })));
}

// ── presence, rest parameters, void ────────────────────────────────────────

#[test]
fn an_optional_parameter_rejects_null_unless_declared() {
    let f = Function::new("f", "function(parameters(x?: number); returns(number))");
    assert_eq!(f.rejected(&[Value::Null]).argument_errors().next().expect("one").problem, ArgumentProblem::NullNotAdmitted);
    let omitted = f.call(&[]).expect("omission binds");
    assert!(omitted.args().is_omitted(0));
    assert_eq!(omitted.args().get(0), None);
}

#[test]
fn omission_and_null_stay_distinct() {
    let f = Function::new("f", "function(parameters(x?: number | null); returns(number))");
    let null = f.call(&[Value::Null]).expect("null binds");
    assert!(!null.args().is_omitted(0));
    assert_eq!(null.args().get(0), Some(&Value::Null));
    let omitted = f.call(&[]).expect("omission binds");
    assert!(omitted.args().is_omitted(0));
}

#[test]
fn any_without_null_rejects_null_at_a_parameter() {
    let f = Function::new("f", "function(parameters(x: any); returns(boolean))");
    f.rejected(&[Value::Null]);
    assert_eq!(f.received(&[json!([null])]), [json!([null])], "only the parameter itself has the null rule");
}

#[test]
fn rest_parameters_bind_each_remaining_argument() {
    let join = Function::new("join", "function(parameters(separator: string, ...parts: number); returns(string))");
    let call = join.call(&[json!(","), json!("1"), json!(2), json!("3")]).expect("binds");
    assert_eq!(call.args().rest().cloned().collect::<Vec<_>>(), [json!(1), json!(2), json!(3)]);
    assert_eq!(join.call(&[json!(",")]).expect("zero rest arguments").args().rest().len(), 0);
    let error = join.rejected(&[json!(","), json!(1), json!("x")]);
    let argument = error.argument_errors().next().expect("one");
    assert_eq!((argument.parameter.as_str(), argument.index), ("parts", 2));
    assert!(error.to_string().contains("parts (argument 3)"), "{error}");
    assert_eq!(join.rejected(&[]).kind(), CallErrorKind::Arity);
}

#[test]
fn void_takes_exactly_zero_arguments() {
    let now = Function::new("now", "function(parameters(void); returns(string))");
    assert!(now.call(&[]).expect("binds").args().is_empty());
    let error = now.rejected(&[json!(1)]);
    assert_eq!(error.kind(), CallErrorKind::Arity);
    assert!(error.to_string().contains("expected 0 arguments, received 1"), "{error}");
}

#[test]
fn too_many_arguments_is_an_arity_error() {
    let min = min();
    let error = min.rejected(&[json!(1), json!(2), json!(3)]);
    assert_eq!(error.kind(), CallErrorKind::Arity);
    assert!(error.to_string().contains("min: no signature takes 3 arguments"), "{error}");
}

// ── laziness ────────────────────────────────────────────────────────────────

#[test]
fn lazy_binding_never_binds_unevaluated_operands() {
    let and = FunctionRegistration::builder("and", "function(parameters(...values: unknown | null); returns(boolean))")
        .evaluation(EvaluationMode::Lazy)
        .build()
        .expect("registration builds");
    let evaluated = Cell::new(0);
    let operands: Vec<Box<dyn Fn() -> Value>> = vec![
        Box::new(|| json!(true)),
        Box::new(|| json!(0)),
        Box::new(|| panic!("a short-circuited operand is never evaluated")),
        Box::new(|| panic!("a short-circuited operand is never evaluated")),
    ];
    let mut call = and.lazy_call().expect("a lazy registration binds incrementally");
    for operand in &operands {
        evaluated.set(evaluated.get() + 1);
        let bound = call.bind_next(operand()).expect("unknown | null binds anything");
        let truthy = !matches!(bound, Value::Bool(false) | Value::Null) && bound != &json!(0);
        if !truthy {
            break;
        }
    }
    assert_eq!((evaluated.get(), call.bound()), (2, 2));
    let call = call.finish().expect("rest arity is satisfied");
    assert_eq!(call.args().rest().cloned().collect::<Vec<_>>(), [json!(true), json!(0)]);
    assert!(and.lazy_call().expect("lazy").finish().expect("zero operands").args().is_empty());
}

#[test]
fn lazy_binding_reports_each_operand_as_it_binds() {
    let all_numbers = FunctionRegistration::builder("all", "function(parameters(...values: number); returns(boolean))")
        .evaluation(EvaluationMode::Lazy)
        .build()
        .expect("registration builds");
    let mut call = all_numbers.lazy_call().expect("lazy");
    assert_eq!(call.bind_next(json!("4")).expect("converts"), &json!(4));
    let error = call.bind_next(json!("pear")).expect_err("not a number");
    assert_eq!(error.argument_errors().next().expect("one").index, 1);
}

#[test]
fn eager_registrations_have_no_lazy_path_and_lazy_ones_have_one_signature() {
    assert!(min().registration.lazy_call().is_none());
    let overloaded = FunctionRegistration::builder("or", format!("{PR_COUNT} | {PR_QUERY}"))
        .evaluation(EvaluationMode::Lazy)
        .build();
    assert!(matches!(overloaded, Err(RegistrationError::InvalidLazy { .. })));
}

// ── return contracts ────────────────────────────────────────────────────────

#[test]
fn wrong_return_reports_implementation_contract_error() {
    let three = Function::new("three", "function(parameters(void); returns(number))");
    let call = three.call(&[]).expect("binds");
    let returned = json!("3");
    let error = three.registration.check_return(&call, &returned).expect_err("text is not a number");
    assert_eq!(error.kind(), CallErrorKind::ImplementationContract);
    let CallBindingError::ImplementationContract(violation) = &error else {
        panic!("expected an implementation-contract error");
    };
    assert_eq!(violation.returned, json!("3"), "the result is never converted");
    assert_eq!(violation.expected, "number");
    assert!(matches!(violation.error.failures[0].reason, FailureReason::ConversionNotAllowed));
    assert!(error.to_string().contains("implementation contract violated"), "{error}");
    assert!(three.registration.check_return(&call, &json!(3)).is_ok());
}

// ── registration rules ──────────────────────────────────────────────────────

#[test]
fn executable_registration_rejects_function_values() {
    let parameter = FunctionRegistration::builder(
        "apply",
        "function(parameters(f: function(parameters(x: number); returns(number)), x: number); returns(number))",
    )
    .build();
    assert!(
        matches!(&parameter, Err(RegistrationError::FunctionValued { path, .. }) if path == "f"),
        "{parameter:?}"
    );
    let result = FunctionRegistration::builder(
        "make",
        "function(parameters(void); returns(function(parameters(void); returns(string))))",
    )
    .build();
    assert!(matches!(&result, Err(RegistrationError::FunctionValued { path, .. }) if path == "returns"), "{result:?}");
}

#[test]
fn a_registration_describes_functions_only() {
    let error = FunctionRegistration::builder("f", "string").build();
    assert!(matches!(error, Err(RegistrationError::NotAFunction { .. })));
    let error = FunctionRegistration::builder("f", "function(").build();
    assert!(matches!(error, Err(RegistrationError::Grammar { .. })));
}

#[test]
fn duplicate_overloads_are_rejected() {
    let written = FunctionRegistration::builder(
        "f",
        "function(parameters(a: number); returns(number)) | function(parameters(b: number); returns(string))",
    )
    .build();
    assert!(matches!(written, Err(RegistrationError::Grammar { .. })), "the parser rejects it: {written:?}");
    let types: Vec<FunctionType> = ["function(parameters(a: number); returns(number))", "function(parameters(b: number); returns(string))"]
        .iter()
        .map(|text| match parse_type_definition("f", text).expect("parses") {
            PropertyDef::Single(atom) => match atom.ty {
                TypeExpr::Function(function) => *function,
                other => panic!("not a function: {other:?}"),
            },
            other => panic!("not one function: {other:?}"),
        })
        .collect();
    let compiled = FunctionRegistration::builder_from_types("f", types).build();
    assert!(matches!(compiled, Err(RegistrationError::DuplicateOverload { first: 0, second: 1, .. })));
}

#[test]
fn two_exact_overloads_are_ambiguous_in_either_order() {
    for text in [
        "function(parameters(x: number); returns(number)) | function(parameters(x: number(integer)); returns(number))",
        "function(parameters(x: number(integer)); returns(number)) | function(parameters(x: number); returns(number))",
    ] {
        let f = Function::new("f", text);
        let error = f.rejected(&[json!(4)]);
        assert!(matches!(&error, CallBindingError::AmbiguousOverload { exact: true, signatures, .. } if signatures.len() == 2));
        let error = f.rejected(&[json!("4")]);
        assert!(matches!(&error, CallBindingError::AmbiguousOverload { exact: false, .. }), "{error}");
    }
}

#[test]
fn an_exact_overload_wins_over_a_converting_one_in_either_order() {
    for text in [
        "function(parameters(x: string); returns(number)) | function(parameters(x: number); returns(number))",
        "function(parameters(x: number); returns(number)) | function(parameters(x: string); returns(number))",
    ] {
        let f = Function::new("f", text);
        let call = f.call(&[json!("4")]).expect("binds");
        assert_eq!(call.args().get(0), Some(&json!("4")));
        assert_eq!(f.registration.overloads()[call.overload()].parameters()[0].type_text(), "string");
        let call = f.call(&[json!(4)]).expect("binds");
        assert_eq!(f.registration.overloads()[call.overload()].parameters()[0].type_text(), "number");
    }
}

#[test]
fn overloads_failing_only_on_arity_report_an_arity_error() {
    let f = Function::new(
        "f",
        "function(parameters(a: number); returns(number)) | function(parameters(a: number, b: number); returns(number))",
    );
    let error = f.rejected(&[]);
    assert_eq!(error.kind(), CallErrorKind::Arity);
    assert!(matches!(&error, CallBindingError::Arity { candidates, .. } if candidates.len() == 2));
}

// ── the grammar translator ─────────────────────────────────────────────────

fn def(text: &str) -> PropertyDef {
    parse_type_definition("p", text).expect("parses")
}

fn bare(text: &str) -> darkmatter::markdown::coercion::Shape {
    shape_from_property_def(&def(text), &NoNamedTypes).expect("translates")
}

#[test]
fn optional_properties_admit_null_and_required_ones_do_not() {
    let optional = property_shape_from_def(&def("number"), &NoNamedTypes).expect("translates");
    assert!(!optional.required);
    assert!(bind(&optional.shape, &Value::Null).is_ok());
    let required = property_shape_from_def(&def("number(required)"), &NoNamedTypes).expect("translates");
    assert!(required.required);
    assert!(bind(&required.shape, &Value::Null).is_err());
    let generated = property_shape_from_def(&def("number(required; generated)"), &NoNamedTypes).expect("translates");
    assert!(!generated.required && bind(&generated.shape, &Value::Null).is_err());
    let file = property_shape_from_def(&def("file"), &NoNamedTypes).expect("translates");
    assert!(bind(&file.shape, &json!("")).is_ok(), "an optional scalar file admits the empty sentinel");
}

#[test]
fn inline_objects_are_closed_unless_they_declare_a_catch_all() {
    let closed = bare("{id: number(required), name: string}");
    assert_eq!(bind(&closed, &json!({"id": "1"})).expect("binds").value(), &json!({"id": 1}));
    assert!(bind(&closed, &json!({"name": "x"})).is_err(), "a required property is missing");
    assert!(bind(&closed, &json!({"id": 1, "extra": 2})).is_err());
    let dictionary = bare("{id: string, <string>: number}");
    assert_eq!(bind(&dictionary, &json!({"id": 7, "n": "2"})).expect("binds").value(), &json!({"id": "7", "n": 2}));
    let prefixed = bare("{<starting::x_>: boolean}");
    assert!(bind(&prefixed, &json!({"x_a": "yes"})).is_ok());
    assert!(bind(&prefixed, &json!({"y": true})).is_err());
}

#[test]
fn refinements_convert_to_text_then_check_their_format() {
    let date = bare("date");
    assert!(bind(&date, &json!("2026-10-22")).is_ok());
    assert!(bind(&date, &json!(2026)).is_err(), "2026 becomes text, then fails the date check");
    let url = bare("url(scheme(https))");
    assert!(bind(&url, &json!("https://example.com")).is_ok());
    assert!(bind(&url, &json!("http://example.com")).is_err());
    assert!(bind(&bare("email"), &json!("ada@example.com")).is_ok());
    assert!(bind(&bare("file(eager)"), &json!("./does-not-exist-anywhere.md")).is_ok(), "existence is never checked passively");
    assert_eq!(bind(&bare(r#"enum("1", "2")"#), &json!(2)).expect("binds").value(), &json!("2"));
    assert!(bind(&bare("alphanumeric"), &json!(42)).is_err(), "no scalar text conversion into alphanumeric");
    assert!(bind(&bare("alphanumeric"), &json!("abc42")).is_ok());
}

#[test]
fn grouped_unions_of_lists_and_tuples_with_spreads_translate() {
    let list = bare("(string | number)[](max(2))");
    assert_eq!(bind(&list, &json!([1, "a"])).expect("binds").value(), &json!([1, "a"]));
    assert!(bind(&list, &json!([1, 2, 3])).is_err());
    let variadic = bare("tuple([string, ...number[]])");
    assert_eq!(bind(&variadic, &json!(["a", "1", "2"])).expect("binds").value(), &json!(["a", 1, 2]));
}

#[test]
fn named_references_resolve_through_the_caller() {
    let resolver = |name: &str, _reference: &str| (name == "Port").then(|| def("number(integer; min(1))"));
    let port = shape_from_property_def(&def("Port"), &resolver).expect("resolves");
    assert_eq!(bind(&port, &json!("80")).expect("binds").value(), &json!(80));
    assert!(bind(&port, &json!(0)).is_err());
    let ports = shape_from_property_def(&def("Port[]"), &resolver).expect("resolves");
    assert_eq!(bind(&ports, &json!(["1", 2])).expect("binds").value(), &json!([1, 2]));
    let missing = shape_from_property_def(&def("Host"), &resolver);
    assert!(matches!(missing, Err(TranslateError::UnresolvedReference { .. })));
    let registration = FunctionRegistration::builder("connect", "function(parameters(port: Port); returns(boolean))")
        .build_with(&resolver)
        .expect("resolves");
    assert_eq!(Function::from(registration).received(&[json!("8080")]), [json!(8080)]);
    let unresolved = FunctionRegistration::builder("connect", "function(parameters(port: Port); returns(boolean))").build();
    assert!(matches!(unresolved, Err(RegistrationError::Translate { .. })));
}

#[test]
fn constraints_the_lowering_rejects_fail_translation() {
    for text in ["boolean(min(1))", "number(min(5); max(1))", "date(max(3))"] {
        assert!(shape_from_property_def(&def(text), &NoNamedTypes).is_err(), "{text}");
    }
}

#[test]
fn callback_mode_accepts_inline_parameters_but_never_stored_values() {
    let signature = "function(parameters(xs: unknown[] | object | null, f: function(parameters(x: unknown | null); returns(unknown | null))); returns((unknown | null)[]))";
    let registration = FunctionRegistration::builder("map", signature)
        .evaluation(EvaluationMode::Callback).build().unwrap();
    let callback = registration.overloads()[0].parameters()[1].callback().unwrap();
    assert_eq!(callback.parameters.as_slice().len(), 1);
    assert_eq!(registration.evaluation(), EvaluationMode::Callback);
    assert!(registration.lazy_call().is_none());
    for value in [json!(null), json!("x => x"), json!({}), json!([]), json!(true), json!(1)] {
        assert!(!registration.overloads()[0].parameters()[1].binds(&value), "{value}");
    }
    let error = FunctionRegistration::builder("make", "function(parameters(void); returns(function(parameters(void); returns(string))))")
        .evaluation(EvaluationMode::Callback).build().unwrap_err();
    assert!(matches!(error, RegistrationError::FunctionValued { path, .. } if path == "returns"));
}

#[test]
fn callback_mode_rejects_callbacks_nested_in_stored_parameters() {
    for signature in [
        "function(parameters(f: function(parameters(x: number); returns(number))[]); returns(number))",
        "function(parameters(f: {stored: function(parameters(x: number); returns(number))}); returns(number))",
    ] {
        let error = FunctionRegistration::builder("apply", signature).evaluation(EvaluationMode::Callback).build();
        assert!(matches!(error, Err(RegistrationError::FunctionValued { .. })), "{error:?}");
    }
}

#[test]
fn shipped_collection_callback_signatures_are_not_lifecycle_verbs() {
    use darkmatter::markdown::compose::expression::function_descriptors;
    use darkmatter::lifecycle::actions::{all_lifecycle_verbs, expression_function_signature, is_known_lifecycle_verb};

    for name in ["map", "filter"] {
        let descriptors = function_descriptors(name);
        assert_eq!(descriptors.len(), 1);
        let descriptor = &descriptors[0];
        assert_eq!(descriptor.parameters.len(), 2);
        assert!(!descriptor.accepts_value_arguments());
        let registration = FunctionRegistration::builder_from_types(name, vec![descriptor.function_type.clone()])
            .evaluation(EvaluationMode::Callback).build().unwrap();
        assert!(registration.overloads()[0].parameters()[1].callback().is_some());
        assert!(registration.overloads()[0].check_return(&json!([null, 1, "x"])).is_ok());
        assert!(expression_function_signature(name).is_none());
        assert!(!is_known_lifecycle_verb(name));
        assert!(!all_lifecycle_verbs().contains(&name));
        let authored = json!({"composed": [{(name): [[], "x => x"]}]});
        let mut options = darkmatter::lifecycle::md_host::parse_options(None);
        options.unknown_verbs = darkmatter::lifecycle::UnknownVerbPolicy::Reject;
        assert!(darkmatter::lifecycle::parse_lifecycle_config_with(&authored, std::path::Path::new("doc.md"), &options).is_err());
    }
}

#[test]
fn callback_runtime_preserves_order_nulls_patterns_and_nested_scopes() {
    use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, parse};
    struct Lookup;
    impl EvaluationLookup for Lookup {
        fn get(&self, path: &str) -> Option<Value> {
            (path == "fm.n" || path == "n").then(|| json!(9))
        }
    }
    for (source, expected) in [
        ("map([1, null, 3], n => n)", json!([1, null, 3])),
        ("filter([0, null, 2, false, '', [], {}], n -> n)", json!([2])),
        ("map({b: 2, a: null}, {key, value} => [key, value])", json!([["b", 2], ["a", null]])),
        ("filter({b: 2, a: null}, entry => entry.value)", json!([{"key": "b", "value": 2}])),
        ("map([1, 2], n => [map([n + 1], n -> n)[0], local.n, fm.n])", json!([[2, 1, 9], [3, 2, 9]])),
        ("map([{v: null}], {v} -> v)", json!([null])),
        ("map(null, {v} => unknown_function(v))", json!([])),
        ("map(missing, n => unknown_function(n))", json!([])),
        ("filter([], n => unknown_function(n))", json!([])),
    ] {
        assert_eq!(evaluate(&parse(source).unwrap(), &Lookup).unwrap(), expected, "{source}");
    }
    assert_eq!(evaluate(&parse("n").unwrap(), &Lookup).unwrap(), json!(9));
}

#[test]
fn callback_runtime_binds_scalars_and_preserves_nested_failure_causes() {
    use darkmatter::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse};
    struct Lookup;
    impl EvaluationLookup for Lookup { fn get(&self, _: &str) -> Option<Value> { None } }
    for source in ["map(3, n => n)", "filter('text', n => n)", "map('[1,2]', n => n)", "map(true, n => n)"] {
        assert!(matches!(evaluate(&parse(source).unwrap(), &Lookup), Err(ExpressionError::Call(_))), "{source}");
    }
    let error = evaluate(&parse("map({team: [1]}, {value} => filter(value, n => unknown_function(n)))").unwrap(), &Lookup).unwrap_err();
    assert!(error.is_authoring_fatal());
    let message = error.to_string();
    assert!(message.contains("map[team]") && message.contains("filter[0]") && message.contains("unknown_function"), "{message}");
    let mut cause: &(dyn std::error::Error + 'static) = &error;
    while let Some(inner) = cause.source() { cause = inner; }
    assert!(matches!(cause.downcast_ref::<ExpressionError>().or_else(|| cause.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref)), Some(ExpressionError::UnknownFunction { .. })));
    for source in ["map([null], {key} => key)", "map([{}], {key} => key)", "map([1], fm => fm)"] {
        assert!(evaluate(&parse(source).unwrap(), &Lookup).is_err(), "{source}");
    }
    assert_eq!(evaluate(&parse("map([1], n => n)").unwrap(), &Lookup).unwrap(), json!([1]));
}

#[test]
fn callback_runtime_evaluates_source_once_and_stops_at_first_error() {
    use std::cell::RefCell;
    use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate, parse};
    struct Lookup { reads: RefCell<Vec<String>> }
    impl EvaluationLookup for Lookup {
        fn get(&self, path: &str) -> Option<Value> {
            self.reads.borrow_mut().push(path.into());
            match path { "items" => Some(json!([1, 2, 3])), "keep" => Some(json!(true)), _ => None }
        }
    }
    let lookup = Lookup { reads: RefCell::new(Vec::new()) };
    assert_eq!(evaluate(&parse("filter(items, n => keep)").unwrap(), &lookup).unwrap(), json!([1, 2, 3]));
    assert_eq!(*lookup.reads.borrow(), ["items", "keep", "keep", "keep"]);
    lookup.reads.borrow_mut().clear();
    assert!(evaluate(&parse("map(items, n => n == 2 ? unknown_function(n) : keep)").unwrap(), &lookup).is_err());
    assert_eq!(*lookup.reads.borrow(), ["items", "keep"]);
}

#[test]
fn callback_runtime_session_budget_is_shared_and_arrays_do_not_charge_bytes() {
    use std::{num::NonZeroU64, sync::Arc};
    use darkmatter::markdown::compose::expression::{BindingView, EvaluationLookup, EvaluationSession, evaluate, parse};
    use darkmatter::markdown::compose::expression::iteration::IterationLimits;
    struct Lookup;
    impl EvaluationLookup for Lookup { fn get(&self, _: &str) -> Option<Value> { None } }
    let session = EvaluationSession::associate(Arc::new(BindingView::baseline()), &Lookup, []).unwrap()
        .with_iteration_limits(IterationLimits::new(NonZeroU64::new(4).unwrap(), NonZeroU64::new(1).unwrap()));
    assert_eq!(evaluate(&parse("map([1, 2], n => filter([n], p => false))").unwrap(), &session).unwrap(), json!([[], []]));
    assert_eq!(session.iteration_budget().usage().visits, 4);
    assert_eq!(session.iteration_budget().usage().bytes, 0);
    let error = evaluate(&parse("map([1], n => n)").unwrap(), &session).unwrap_err();
    assert!(error.is_authoring_fatal());
    assert!(error.to_string().contains("visit"));
}

#[test]
fn callback_runtime_call_binding_severity_survives_the_wrapper() {
    use darkmatter::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse};
    struct Lookup;
    impl EvaluationLookup for Lookup { fn get(&self, _: &str) -> Option<Value> { None } }
    let error = evaluate(&parse("map([true], n => min(n, 1))").unwrap(), &Lookup).unwrap_err();
    assert!(matches!(error.without_callbacks(), ExpressionError::Call(_)));
    assert!(!error.is_authoring_fatal(), "{error}");
    assert_eq!(error.callback_path().to_string(), "map[0]");
}
