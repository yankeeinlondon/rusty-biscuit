//! The three schema checks that read expression text are syntax-only: an
//! expression-typed value, an `expression` default, and a function example's
//! call and expected literal are parsed by `simplified_schema::expression`
//! and never evaluated, resolved, or looked up. A call to a function nothing
//! defines (`unknown_fn(1)`) is therefore accepted; malformed text is not.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::simplified::ExampleOutcome;
use darkmatter::markdown::schemas::simplified::grammar::parse_type_definition;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, PropertyDef, SchemaError, TypeExpr, ValidationReport, parse_yaml_schema, to_json_schema,
};
use serde_json::json;

use crate::request_support::context_at;

/// Validates a one-property document (`when: <value>` against
/// `when: <type_text>`) read-only, anchored in its own temp dir.
fn validate_when(type_text: &str, value: &str) -> ValidationReport {
    let dir = tempfile::tempdir().expect("temp dir");
    let document = format!("---\n$schema:\n  when: \"{type_text}\"\nwhen: '{value}'\n---\nBody\n");
    let markdown = Markdown::from(document).with_source(ComposeSource::File(dir.path().join("doc.md")));
    DarkmatterSchemas::new(context_at(dir.path()))
        .validate(&markdown)
        .expect("schema must compile and validation must run")
}

/// Lowers the one-property schema `when: <type_text>`.
fn lower_when(type_text: &str) -> Result<serde_json::Value, SchemaError> {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&format!("when: \"{type_text}\"\n")).expect("schema YAML");
    to_json_schema(&parse_yaml_schema(&yaml)?)
}

#[test]
fn an_expression_typed_value_calling_an_unknown_function_validates() {
    let report = validate_when("expression(required)", "unknown_fn(1)");
    assert!(report.valid, "{:?}", report.problems);

    let report = validate_when("expression(required)", "unknown_fn(");
    assert!(!report.valid, "a malformed call must fail");
    assert!(
        report.problems.iter().any(|problem| format!("{problem:?}").contains("darkmatter-expression")),
        "the problem names the expression format: {:?}",
        report.problems
    );
}

#[test]
fn an_expression_default_calling_an_unknown_function_lowers() {
    let schema = lower_when("expression(default('unknown_fn(1)'))").expect("the default parses");
    assert_eq!(schema["properties"]["when"]["default"], json!("unknown_fn(1)"));

    match lower_when("expression(default('unknown_fn('))") {
        Err(SchemaError::Convert { property, message }) => {
            assert_eq!(property, "when");
            assert!(message.contains("is not a valid Darkmatter expression"), "{message}");
        }
        other => panic!("a malformed default must fail to lower, got {other:?}"),
    }
}

#[test]
fn a_function_example_calling_an_unknown_function_parses() {
    let text = "function(parameters(void); returns(number); example(unknown_fn(1) => {ok: [1, 2.5]}))";
    let PropertyDef::Single(atom) = parse_type_definition("p", text).expect("example parses") else {
        panic!("one atom");
    };
    let TypeExpr::Function(function) = &atom.ty else { panic!("function") };
    assert_eq!(function.examples.len(), 1);
    assert_eq!(function.examples[0].call, "unknown_fn(1)");
    assert_eq!(&text[function.examples[0].call_span.clone()], "unknown_fn(1)");
    assert_eq!(function.examples[0].expected, ExampleOutcome::Value(json!({ "ok": [1, 2.5] })));

    let grammar_message = |text: &str| match parse_type_definition("p", text) {
        Err(SchemaError::Grammar { message, .. }) => message,
        other => panic!("{text}: expected a grammar error, got {other:?}"),
    };
    let malformed = grammar_message("function(parameters(void); returns(number); example(unknown_fn(1,) => 1))");
    assert!(malformed.contains("example call `unknown_fn(1,)` is not a valid expression"), "{malformed}");
    // An unknown call on the expected side is still a call, not a literal; the
    // check rejects it without evaluating it.
    let computed = grammar_message("function(parameters(void); returns(number); example(unknown_fn(1) => unknown_fn(2)))");
    assert!(computed.contains("an example result must be a literal"), "{computed}");
}
