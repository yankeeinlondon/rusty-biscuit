//! What passive consumers report for a template: type queries, suggestion
//! queries, lint, completion, and the subtype relation see the literal
//! elements and an explicit pending marker, never a partial list presented
//! as complete and never a placeholder.

use serde_json::{Value, json};
use simplified_schema::completion::{AlternativeSite, alternatives_for_def};
use simplified_schema::simplified::grammar::parse_type_expr;
use simplified_schema::simplified::{
    PropertyAtom, PropertyDef, SchemaRoot, lint_suggestions, parse_yaml_schema, suggestions_for_def, suggestions_for_path,
};
use simplified_schema::{SimplifiedSchema, is_subtype};

fn atom(text: &str) -> PropertyAtom {
    parse_type_expr("p", text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn root(yaml: &str) -> SchemaRoot {
    parse_yaml_schema(&serde_yaml_ng::from_str(yaml).unwrap()).unwrap()
}

#[test]
fn a_type_query_reports_the_slots_of_the_type_and_of_its_named_type() {
    let schema = SimplifiedSchema::try_from(include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml")).unwrap();
    let property = schema.property("os_support").expect("declared");
    let expressions: Vec<String> = property.pending_slots().iter().map(|slot| slot.expression.clone()).collect();
    assert_eq!(expressions, vec![r#"load_data("./os.yaml", "string[]")"#.to_string()]);
    assert_eq!(schema.named_type("os").expect("a named type").pending_slots().len(), 1);

    let concrete = SimplifiedSchema::try_from("$schema:\n  os: enum(a, b)\n").unwrap();
    assert!(concrete.property("os").unwrap().pending_slots().is_empty());
}

#[test]
fn a_suggestion_query_offers_the_literals_and_marks_the_list_pending() {
    let schema = root("platform: string(suggest(FreeBSD, {{ load_data('./os.yaml') }}, Plan9))\n");
    let query = suggestions_for_path(&schema, &["platform"]).expect("the property has a list");
    let labels: Vec<&str> = query.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, vec!["FreeBSD", "Plan9"]);
    assert!(query.pending, "the slot's candidates are not known yet");

    let static_query = suggestions_for_def("p", &PropertyDef::Single(atom("string(suggest(a, b))"))).unwrap();
    assert!(!static_query.pending);
    let dynamic_query = suggestions_for_def("p", &PropertyDef::Single(atom("string(suggest({{ s }}))"))).unwrap();
    assert!(dynamic_query.items.is_empty() && dynamic_query.pending, "a list of only a slot is pending, not absent");
}

#[test]
fn lint_judges_only_the_literal_candidates_it_can_judge() {
    let reasons = |yaml: &str| -> Vec<String> {
        lint_suggestions(&root(yaml)).unwrap().into_iter().map(|problem| problem.decoded).collect()
    };
    assert_eq!(reasons("n: number(suggest(abc, {{ x }}, 2))\n"), vec!["abc".to_string()]);
    assert_eq!(reasons("n: number(suggest(9, {{ x }}); max(5))\n"), vec!["9".to_string()]);
    // A slot that shapes the target leaves nothing to judge against.
    assert!(reasons("n: number(suggest(9, abc); max({{ m }}))\n").is_empty());
}

#[test]
fn completion_offers_literal_members_and_marks_the_values_pending() {
    let completion = |text: &str| {
        alternatives_for_def(&PropertyDef::Single(atom(text)), AlternativeSite::Value, &Value::Null).expect("a value site")
    };
    let template = completion("enum(none, {{ load_data('./os.yaml') }}; required)");
    let labels: Vec<&str> = template.values.iter().map(|value| value.label.as_str()).collect();
    assert_eq!(labels, vec!["none"]);
    assert!(template.pending);

    let literal = completion("literal({{ revision }})");
    assert!(literal.values.is_empty() && literal.pending);

    let concrete = completion("enum(none, macOS)");
    assert_eq!(concrete.values.len(), 2);
    assert!(!concrete.pending);
}

#[test]
fn an_unresolved_enum_is_no_subtype_of_another_type() {
    assert!(!is_subtype(&atom("enum({{ a }})"), &atom("enum({{ b }})")), "two templates are not one type");
    assert!(!is_subtype(&atom("enum(x)"), &atom("enum(x, {{ more }})")), "its members are not known");
    assert!(!is_subtype(&atom("enum(x, {{ more }})"), &atom("enum(x, y)")));
    assert!(!is_subtype(&atom("literal({{ v }})"), &atom("string")));
    assert!(is_subtype(&atom("enum(x)"), &atom("enum(x, y)")), "control: a static enum still compares");
}

#[test]
fn validating_against_a_template_never_reports_success() {
    let schema = SimplifiedSchema::try_from("$schema:\n  os: enum(a, {{ x }})\n").unwrap();
    for value in [json!({ "os": "a" }), json!({}), json!({ "os": 5 })] {
        assert!(schema.validate_strict(&value).is_err(), "{value}");
        assert!(schema.validate_soft(&value).is_err(), "{value}");
    }
}
