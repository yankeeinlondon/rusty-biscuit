//! Passive operand validation (`darkmatter::lifecycle::validate_operands`)
//! through the public API: `{{ … }}` operands are deferred, every literal
//! operand binds exactly as the runtime registration binds it, and nothing is
//! evaluated or run.

use std::cell::Cell;

use darkmatter::lifecycle::executor::is_template_operand;
use darkmatter::lifecycle::{OperandVerdict, validate_expression_function_operands, validate_operands, validate_operands_with};
use darkmatter::markdown::coercion::PathSegment;
use darkmatter::markdown::compose::expression::call_binding::{
    ArgumentProblem, CallBindingError, CallErrorKind, FunctionRegistration, OverloadSelection,
};
use darkmatter::markdown::compose::expression::functions::function_registration;
use darkmatter::markdown::compose::expression::{Expr, defer_templates};
use darkmatter::markdown::compose::subtree::SubtreeCompose;
use darkmatter::markdown::compose::{ComposeContext, EffectiveStateBuilder};
use serde_json::{Value, json};
use tempfile::TempDir;

fn verdict(verb: &str, args: &[Value]) -> OperandVerdict {
    validate_expression_function_operands(verb, args).unwrap_or_else(|| panic!("{verb} is not registered"))
}

/// The rejection's error, panicking on any other verdict.
fn rejected(verdict: OperandVerdict) -> CallBindingError {
    match verdict {
        OperandVerdict::Rejected(error) => error,
        other => panic!("expected a rejection, got {other:?}"),
    }
}

/// The argument indices every failing argument names.
fn failing_indices(error: &CallBindingError) -> Vec<usize> {
    let mut indices: Vec<usize> = error.argument_errors().map(|error| error.index).collect();
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn deferred(overloads: &[usize]) -> OperandVerdict {
    OperandVerdict::Deferred {
        overloads: overloads.to_vec(),
    }
}

// ── pr_list: (query: object) | (count: number(integer; min(1); max(100))) ──

#[test]
fn pr_list_known_count_binds_the_count_overload() {
    assert_eq!(verdict("pr_list", &[json!(3)]), OperandVerdict::Bound { overload: 1 });
    assert_eq!(verdict("pr_list", &[json!("3")]), OperandVerdict::Bound { overload: 1 });
}

#[test]
fn pr_list_rejects_a_literal_that_no_overload_accepts() {
    let error = rejected(verdict("pr_list", &[json!("three")]));
    assert_eq!(error.kind(), CallErrorKind::Type);
    assert_eq!(failing_indices(&error), [0]);

    for out_of_range in [json!(0), json!(101)] {
        let error = rejected(verdict("pr_list", std::slice::from_ref(&out_of_range)));
        assert_eq!(error.kind(), CallErrorKind::Type, "{out_of_range}");
        assert_eq!(failing_indices(&error), [0], "{out_of_range}");
    }
}

#[test]
fn pr_list_template_count_leaves_both_overloads_open() {
    assert_eq!(verdict("pr_list", &[json!("{{ fm.count }}")]), deferred(&[0, 1]));
}

// ── siblings, arity, and presence are never deferred ───────────────────────

#[test]
fn min_with_a_deferred_operand_checks_its_literal_sibling() {
    assert_eq!(verdict("min", &[json!(1), json!("{{ fm.x }}")]), deferred(&[0]));

    let error = rejected(verdict("min", &[json!("three"), json!("{{ fm.x }}")]));
    assert_eq!(failing_indices(&error), [0]);
    assert_eq!(error.argument_errors().next().unwrap().parameter, "a");
}

#[test]
fn arity_is_never_deferred() {
    let extra = rejected(verdict("min", &[json!(1), json!("{{ fm.x }}"), json!(2)]));
    assert_eq!(extra.kind(), CallErrorKind::Arity);

    let missing = rejected(verdict("min", &[json!("{{ fm.x }}")]));
    assert_eq!(missing.kind(), CallErrorKind::Arity);
}

#[test]
fn optional_non_nullable_parameter_rejects_null_and_admits_omission() {
    let error = rejected(verdict("ping", &[json!("192.168.1.1"), Value::Null]));
    let argument = error.argument_errors().next().expect("one failing argument");
    assert_eq!((argument.index, &argument.problem), (1, &ArgumentProblem::NullNotAdmitted));

    assert_eq!(verdict("ping", &[json!("192.168.1.1")]), OperandVerdict::Bound { overload: 0 });
    assert_eq!(verdict("ping", &[json!("192.168.1.1"), json!("{{ fm.timeout }}")]), deferred(&[0]));
}

// ── structured operands defer per child ─────────────────────────────────────

#[test]
fn array_operand_defers_only_its_template_children() {
    let total = FunctionRegistration::builder("total", "function(parameters(counts: number[]); returns(number))")
        .build()
        .unwrap();

    assert_eq!(validate_operands(&total, &[json!(["1", "{{ fm.x }}", 3])]), deferred(&[0]));

    let error = rejected(validate_operands(&total, &[json!(["1", "{{ fm.x }}", "three"])]));
    let argument = error.argument_errors().next().expect("one failing argument");
    assert_eq!(argument.index, 0);
    assert_eq!(argument.paths(), ["counts[2]"]);
}

#[test]
fn object_operand_defers_only_its_template_properties() {
    let query = FunctionRegistration::builder(
        "query",
        "function(parameters(filter: { state: string, limit: number }); returns(number))",
    )
    .build()
    .unwrap();

    assert_eq!(
        validate_operands(&query, &[json!({"state": "{{ fm.s }}", "limit": "5"})]),
        deferred(&[0])
    );

    let error = rejected(validate_operands(&query, &[json!({"state": "{{ fm.s }}", "limit": "five"})]));
    assert_eq!(error.argument_errors().flat_map(|argument| argument.paths()).collect::<Vec<_>>(), ["filter.limit"]);
}

#[test]
fn untyped_object_operand_is_bound_without_consulting_its_children() {
    // `pr_list`'s query is a bare `object`: no property is declared, so a
    // template inside it decides nothing and the call binds now.
    assert_eq!(
        verdict("pr_list", &[json!({"state": "{{ fm.s }}"})]),
        OperandVerdict::Bound { overload: 0 }
    );
}

// ── lazy functions ──────────────────────────────────────────────────────────

#[test]
fn lazy_variadic_functions_validate_every_operand() {
    assert_eq!(verdict("and", &[json!(true), json!("{{ fm.ready }}")]), deferred(&[0]));
    assert_eq!(verdict("or", &[json!("{{ fm.a }}"), json!("{{ fm.b }}")]), deferred(&[0]));
    assert_eq!(verdict("and", &[json!(true), json!(false)]), OperandVerdict::Bound { overload: 0 });
    assert_eq!(verdict("or", &[]), OperandVerdict::Bound { overload: 0 });
}

// ── what is and is not a template ───────────────────────────────────────────

#[test]
fn malformed_and_unclosed_templates_are_judged_as_literals() {
    for literal in ["{{ 1 + }}", "{{ fm.count"] {
        let error = rejected(verdict("pr_list", &[json!(literal)]));
        assert_eq!(failing_indices(&error), [0], "{literal}");
        assert!(
            error.argument_errors().all(|argument| argument.received == json!(literal)),
            "{literal}: {error}"
        );
    }
}

#[test]
fn mixed_text_template_is_deferred() {
    assert_eq!(verdict("pr_list", &[json!("1{{ fm.digit }}")]), deferred(&[0, 1]));
}

// ── the runtime registration, not a bare one ────────────────────────────────

#[test]
fn source_exclusions_come_from_the_runtime_registration() {
    let error = rejected(verdict("length", &[json!(true)]));
    let argument = error.argument_errors().next().expect("one failing argument");
    assert_eq!(argument.problem, ArgumentProblem::SourceExcluded);

    let runtime = function_registration("length").expect("registered");
    assert_eq!(runtime.name(), "length");
    assert!(std::ptr::eq(runtime, function_registration("LENGTH").unwrap()));
}

#[test]
fn unknown_verb_has_no_registration() {
    assert_eq!(validate_expression_function_operands("no_such_function", &[]), None);
    assert!(function_registration("no_such_function").is_none());
}

// ── passivity ───────────────────────────────────────────────────────────────

#[test]
fn validation_consults_only_the_deferral_predicate() {
    let consulted = Cell::new(0);
    let counting = |path: &[PathSegment], value: &Value| {
        consulted.set(consulted.get() + 1);
        defer_templates(path, value)
    };

    // `ping` is a context function whose handler sends ICMP; validation
    // must decide without it.
    let ping = function_registration("ping").expect("registered");
    let result = validate_operands_with(ping, &[json!("{{ fm.host }}"), json!(250)], &counting);

    assert_eq!(result, deferred(&[0]));
    assert!(consulted.get() >= 2, "every operand is offered to the predicate");
}

#[test]
fn select_overload_reports_open_overloads_while_bind_call_with_stays_ambiguous() {
    let pr_list = function_registration("pr_list").unwrap();
    let args = [json!("{{ fm.count }}")];
    let options = darkmatter::markdown::coercion::BindOptions {
        defer: Some(&defer_templates),
        ..Default::default()
    };

    match pr_list.select_overload_with(&args, &options).unwrap() {
        OverloadSelection::Deferred(calls) => {
            assert_eq!(calls.iter().map(|call| call.overload()).collect::<Vec<_>>(), [0, 1]);
            assert!(calls.iter().all(|call| call.deferred_arguments() == [0]));
        }
        other => panic!("expected a deferred selection, got {other:?}"),
    }

    let error = pr_list.bind_call_with(&args, &options).unwrap_err();
    assert_eq!(error.kind(), CallErrorKind::AmbiguousOverload);

    // Without a predicate the same text is a literal that binds nowhere.
    assert_eq!(pr_list.bind_call(&args).unwrap_err().kind(), CallErrorKind::Type);
}

// ── the executor's template test uses the same predicate ───────────────────

#[test]
fn template_operand_uses_the_shared_template_predicate() {
    let template = |text: &str| is_template_operand(&Expr::StringLiteral(text.to_string()));

    assert!(template("{{ fm.count }}"));
    assert!(template("n={{ fm.count }}"));
    assert!(!template("{{ fm.count"));
    assert!(!template("\\{{ fm.count }}"));
    assert!(!template("{{ 1 + }}"));
    assert!(!template("plain"));
    assert!(!is_template_operand(&Expr::NumberLiteral(3.0)));

    // The compose pass still owns interpolation literals and literal tokens.
    assert!(template("{{{ count }}}"));
    assert!(template("{{!data:abc}}"));
}

/// Text holding `{{` that the scanner recognizes as no span, interpolation
/// literal, or token interpolates to itself, so judging it a literal rather
/// than a template changes no operand's value.
#[test]
fn text_without_a_span_interpolates_verbatim() {
    let dir = TempDir::new().unwrap();
    let state = EffectiveStateBuilder::new()
        .with_frontmatter([("count".to_string(), json!(3))].into())
        .with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .build()
        .unwrap();

    for text in ["{{ fm.count", "n={{ fm.count", "\\{{ count }}"] {
        assert!(!is_template_operand(&Expr::StringLiteral(text.to_string())), "{text}");
        let composed = SubtreeCompose::new(&json!(text), &state).compose().expect(text);
        assert_eq!(composed, json!(text));
    }
}
