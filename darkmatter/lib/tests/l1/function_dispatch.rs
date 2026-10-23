//! Expression-function dispatch through `evaluate(parse(...))`: every call
//! binds before its handler runs, a result is held to its declared return
//! type, and a lazy operand that is never needed is never entered. Test
//! functions and handler-entry counters come from
//! `expression::function_test_hooks` (`effects-instrumentation`).

use std::collections::HashSet;
use std::sync::Arc;

use darkmatter::markdown::compose::expression::call_binding::{CallBindingError, CallErrorKind, FunctionRegistration};
use darkmatter::markdown::compose::expression::function_test_hooks::{self as hooks, TestFunctionGuard};
use darkmatter::markdown::compose::expression::functions::dispatchable_canonical_names;
use darkmatter::markdown::compose::expression::{
    EvaluationLookup, ExpressionError, evaluate, expression_function_catalog, parse,
};
use serde_json::{Value, json};

struct NoValues;

impl EvaluationLookup for NoValues {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }
}

fn run(expression: &str) -> Result<Value, ExpressionError> {
    evaluate(&parse(expression).expect("the expression parses"), &NoValues)
}

fn register(name: &str, signature: &str, handler: hooks::TestHandler) -> TestFunctionGuard {
    let registration = FunctionRegistration::builder(name, signature).build().expect("the test signature compiles");
    hooks::register(registration, handler)
}

fn doubler() -> TestFunctionGuard {
    register(
        "t_double",
        "function(parameters(x: number); returns(number))",
        Arc::new(|call| Ok(json!(call.args().number(0).expect("bound number") * 2.0))),
    )
}

fn call_error(error: &ExpressionError) -> &CallBindingError {
    match error {
        ExpressionError::Call(error) => error,
        other => panic!("expected ExpressionError::Call, got {other:?}"),
    }
}

#[test]
fn a_type_error_never_enters_the_handler() {
    let _guard = doubler();
    hooks::reset_handler_entries();

    let error = run("t_double('pear')").unwrap_err();

    assert_eq!(call_error(&error).kind(), CallErrorKind::Type, "{error}");
    assert!(!error.is_authoring_fatal());
    assert_eq!(hooks::handler_entries("t_double"), 0);
}

#[test]
fn an_accepted_call_enters_its_handler_once() {
    let _guard = doubler();
    hooks::reset_handler_entries();

    assert_eq!(run("t_double('4')").unwrap(), json!(8.0));
    assert_eq!(hooks::handler_entries("t_double"), 1);
}

#[test]
fn a_wrong_return_is_an_implementation_contract_error_and_is_not_converted() {
    let _guard = register("t_wrong", "function(parameters(void); returns(number))", Arc::new(|_| Ok(json!("5"))));
    hooks::reset_handler_entries();

    let error = run("t_wrong()").unwrap_err();

    let CallBindingError::ImplementationContract(violation) = call_error(&error) else {
        panic!("expected an implementation-contract error, got {error:?}");
    };
    assert_eq!(violation.returned, json!("5"), "the result is reported as returned");
    assert!(error.is_authoring_fatal());
    assert_eq!(hooks::handler_entries("t_wrong"), 1);
}

#[test]
fn a_skipped_lazy_operand_is_never_entered() {
    let _guard = register("effect", "function(parameters(void); returns(boolean))", Arc::new(|_| Ok(json!(true))));

    for (expression, expected, entries) in [
        ("and(false, effect())", false, 0),
        ("or(true, effect())", true, 0),
        ("and(true, effect())", true, 1),
    ] {
        hooks::reset_handler_entries();
        assert_eq!(run(expression).unwrap(), json!(expected), "{expression}");
        assert_eq!(hooks::handler_entries("effect"), entries, "{expression}");
    }
}

#[test]
fn a_built_in_entry_counts_under_its_canonical_name() {
    hooks::reset_handler_entries();

    assert_eq!(run("isString('x')").unwrap(), json!(true));

    assert_eq!(hooks::handler_entries("is_string"), 1);
}

#[test]
fn the_embedded_catalog_and_the_registry_have_the_same_functions() {
    let cataloged: HashSet<&str> =
        expression_function_catalog().descriptors.iter().map(|descriptor| descriptor.name).collect();
    let registered: HashSet<&str> = dispatchable_canonical_names().into_iter().collect();
    assert_eq!(cataloged, registered);
}
