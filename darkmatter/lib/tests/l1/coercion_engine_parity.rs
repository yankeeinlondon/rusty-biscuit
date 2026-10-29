//! One engine (criterion 35): frontmatter composition, function-call
//! binding, and normal dispatch (`evaluate(parse(..))` of a test-registered
//! function, with `effects-instrumentation`) reach identical keep/convert/error
//! outcomes over the same value/type pairs, and the old frontmatter-only rule
//! code is gone.

use std::path::PathBuf;

use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};

use crate::request_support::request;

#[derive(Debug, PartialEq)]
enum Outcome {
    Kept,
    Converted(Value),
    Rejected,
}

/// Composes a document whose `$schema` declares `v: <ty>` and whose
/// frontmatter holds `v: <value>`.
fn through_frontmatter(ty: &str, value: &Value) -> Outcome {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    let path: PathBuf = root.join("doc.md");
    let schema = serde_yaml_ng::to_string(&json!({ "$schema": { "v": ty } })).expect("schema YAML");
    let data = serde_yaml_ng::to_string(&json!({ "v": value })).expect("data YAML");
    std::fs::write(&path, format!("---\n{schema}{data}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path()).expect("document loads");
    match markdown.compose_with(&request(ComposeOptions::new().with_source_file(&path))) {
        Ok((composed, _)) => {
            let held = serde_json::to_value(composed.frontmatter().as_map()).expect("frontmatter JSON")["v"].clone();
            if &held == value { Outcome::Kept } else { Outcome::Converted(held) }
        }
        Err(MarkdownError::SchemaValidationFailed { .. }) => Outcome::Rejected,
        Err(other) => panic!("`{ty}` with {value}: unexpected composition error: {other}"),
    }
}

/// Binds `value` as the one argument of a test-registered `f(v: <ty>)`.
fn through_call(ty: &str, value: &Value) -> Outcome {
    let registration = FunctionRegistration::builder("f", format!("function(parameters(v: {ty}); returns(unknown))"))
        .build()
        .expect("the signature compiles");
    let arguments = [value.clone()];
    match registration.bind_call(&arguments) {
        Ok(call) if call.is_exact() => Outcome::Kept,
        Ok(call) => Outcome::Converted(call.args().get(0).expect("the argument is bound").clone()),
        Err(_) => Outcome::Rejected,
    }
}

/// Calls a test-registered `f(v: <ty>)` through normal dispatch with `v`
/// read from a variable; the handler records what it received, and a
/// rejected call must never enter it.
#[cfg(feature = "effects-instrumentation")]
fn through_dispatch(ty: &str, value: &Value) -> Outcome {
    use std::sync::{Arc, Mutex};

    use darkmatter::markdown::compose::expression::function_test_hooks as hooks;
    use darkmatter::markdown::compose::expression::{EvaluationLookup, ExpressionError, evaluate, parse};

    struct Variable(Value);

    impl EvaluationLookup for Variable {
        fn get(&self, path: &str) -> Option<Value> {
            (path == "v").then(|| self.0.clone())
        }
    }

    let registration = FunctionRegistration::builder("f", format!("function(parameters(v: {ty}); returns(boolean))"))
        .build()
        .expect("the signature compiles");
    let received = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&received);
    let _guard = hooks::register(
        registration,
        Arc::new(move |call| {
            *sink.lock().expect("recorder") = call.args().get(0).cloned();
            Ok(Value::Bool(true))
        }),
    );
    hooks::reset_handler_entries();
    let result = evaluate(&parse("f(v)").expect("the expression parses"), &Variable(value.clone()));
    let entries = hooks::handler_entries("f");
    match result {
        Ok(_) => {
            assert_eq!(entries, 1, "`{ty}` with {value}: an accepted call enters the handler once");
            let held = received.lock().expect("recorder").take().expect("the handler received the argument");
            if &held == value { Outcome::Kept } else { Outcome::Converted(held) }
        }
        Err(ExpressionError::Call(_)) => {
            assert_eq!(entries, 0, "`{ty}` with {value}: a rejected call never enters the handler");
            Outcome::Rejected
        }
        Err(other) => panic!("`{ty}` with {value}: unexpected dispatch error: {other}"),
    }
}

fn assert_one_engine(ty: &str, values: &[Value]) {
    let mut disagreements = Vec::new();
    for value in values {
        let frontmatter = through_frontmatter(ty, value);
        let call = through_call(ty, value);
        if frontmatter != call {
            disagreements.push(format!("{value}: frontmatter {frontmatter:?}, call {call:?}"));
        }
        #[cfg(feature = "effects-instrumentation")]
        {
            let dispatch = through_dispatch(ty, value);
            if frontmatter != dispatch {
                disagreements.push(format!("{value}: frontmatter {frontmatter:?}, dispatch {dispatch:?}"));
            }
        }
    }
    assert!(disagreements.is_empty(), "`{ty}`:\n{}", disagreements.join("\n"));
}

macro_rules! parity {
    ($($name:ident: $ty:expr => [$($value:tt),+ $(,)?];)+) => {
        $(
            #[test]
            fn $name() {
                assert_one_engine($ty, &[$(json!($value)),+]);
            }
        )+
    };
}

parity! {
    string_pairs: "string" => [1.2, "x", true, 2026];
    boolean_pairs: "boolean" => ["yes", "On", "maybe", 1, false];
    number_pairs: "number" => ["+4", "1_000", "99999999999999999999", "9007199254740993", "pear", true, 4];
    integer_pairs: "number(integer)" => ["2.5", "4", "1e20"];
    numberlike_pairs: "numberlike" => ["02134", " 4 ", "pear", 5, true];
    boolish_pairs: "boolish" => ["On", "Yes", 1, "maybe"];
    enum_pairs: "enum(\"1\", \"2\")" => [2, "2", 3];
    json_pairs: "json" => [{"a": 1}, "{\"a\":1}", "{a"];
    content_union_pairs: "json | yaml" => [{"port": 8080, "debug": true}, "port: 8080"];
    nested_object_pairs: "{ inner: { count: number } }" => [{"inner": {"count": "3"}}, {"inner": {"count": "x"}}];
    list_union_pairs: "(boolean | number)[]" => [["2", "yes"], ["maybe"]];
    scalar_union_pairs: "number | boolean" => ["1", "yes", "pear"];
    text_or_list_pairs: "string | string[]" => [2026, ["a", 1]];
    constrained_union_pairs: "number(min(5)) | boolean" => ["1", "7"];
    tuple_pairs: "tuple([string, number, boolean?])" => [["Ada", "36", "yes"], ["Ada", "36"], ["Ada"]];
    dictionary_pairs: "{ <string>: number }" => [{"a": "3", "b": 4}, {"a": "x"}];
    crossed_union_pairs: "{ a: number, b: boolean } | { a: boolean, b: number }" => [{"a": "1", "b": "1"}];
}

fn assert_entry_points_outcome(ty: &str, value: &Value, expected: Outcome) {
    assert_eq!(through_frontmatter(ty, value), expected, "frontmatter: {ty} with {value}");
    assert_eq!(through_call(ty, value), expected, "binding: {ty} with {value}");
    #[cfg(feature = "effects-instrumentation")]
    assert_eq!(through_dispatch(ty, value), expected, "dispatch: {ty} with {value}");
}

#[test]
fn variadic_tuple_constraints_agree_through_composition_and_dispatch() {
    let ty = "tuple([string, ...number(integer; min(1))[]])";
    for (value, expected) in [
        (json!(["head"]), Outcome::Kept),
        (json!(["head", 1, 2]), Outcome::Kept),
        (json!(["head", "1", "2"]), Outcome::Converted(json!(["head", 1, 2]))),
        (json!(["head", "1", "2.5"]), Outcome::Rejected),
        (json!(["head", "1", "0"]), Outcome::Rejected),
        (json!(["head", "pear"]), Outcome::Rejected),
        (json!([]), Outcome::Rejected),
        (json!("head"), Outcome::Rejected),
    ] {
        assert_entry_points_outcome(ty, &value, expected);
    }
}

#[test]
fn nested_tuple_spreads_keep_optional_slots_distinct_from_null() {
    let ty = "tuple([{ id: number(required) }, ...tuple([boolean, number?])[]])";
    for (value, expected) in [
        (json!([{"id": 7}]), Outcome::Kept),
        (
            json!([{"id": "7"}, ["yes", "3"], ["off"]]),
            Outcome::Converted(json!([{"id": 7}, [true, 3], [false]])),
        ),
        (json!([{"id": "7"}, ["yes", null]]), Outcome::Rejected),
        (json!([{"id": "7"}, []]), Outcome::Rejected),
        (json!([{}, [true]]), Outcome::Rejected),
    ] {
        assert_entry_points_outcome(ty, &value, expected);
    }
}

/// The frontmatter-only rule list (`CoercionTarget`, the regex number test,
/// the first-arm root-union and exactly-one property-union pickers) is
/// deleted; frontmatter goes through `markdown::coercion`.
#[test]
fn old_frontmatter_rule_code_is_gone() {
    let source = include_str!("../../src/markdown/schemas/coerce.rs");
    for retired in [
        "CoercionTarget",
        "fn coercion_target",
        "fn is_numberlike",
        "fn coerce_to_number",
        "fn coerce_to_boolean",
        "fn coerce_root_union",
        "fn coerce_property_union",
        "BOOLISH_VALUES",
        "NUMBERLIKE_PATTERN",
    ] {
        assert!(!source.contains(retired), "schemas/coerce.rs still holds `{retired}`");
    }
    assert!(source.contains("coercion::"), "frontmatter conversion must go through the core");
}

/// Control: each side observes all three outcomes, so agreement above is
/// not two harnesses failing the same way.
#[test]
fn both_sides_observe_keep_convert_and_reject() {
    #[cfg(not(feature = "effects-instrumentation"))]
    let sides = [through_frontmatter as fn(&str, &Value) -> Outcome, through_call];
    #[cfg(feature = "effects-instrumentation")]
    let sides = [through_frontmatter as fn(&str, &Value) -> Outcome, through_call, through_dispatch];
    for side in sides {
        assert_eq!(side("string", &json!("x")), Outcome::Kept);
        assert_eq!(side("number", &json!("+4")), Outcome::Converted(json!(4)));
        assert_eq!(side("number", &json!("pear")), Outcome::Rejected);
    }
}
