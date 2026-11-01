//! The `non-null` presence marker through the public API: an absent key is
//! valid, an explicit `null` is rejected at its pointer under both strict
//! and soft validation (and is never converted or read as absent), empty
//! values are judged by the type, the property stays out of the exported
//! `required` list, and a `non-null` over a null-admitting type is a
//! declaration error wherever it is found.
//!
//! The grammar's accepted and rejected spellings are in
//! `declaration_grammar.rs`.

use serde_json::{Value, json};
use simplified_schema::simplified::grammar::parse_type_expr;
use simplified_schema::simplified::{PropertyDef, parse_property_definition};
use simplified_schema::{
    SchemaError, SimplifiedSchema, SimplifiedSchemaError, ValidationFailure, ValidationProblemCode,
    ValidationProblemKind, ValidationSuccess, is_definition_subtype, is_subtype,
};

/// One property of each field shape, marked, beside an unmarked control.
const SCHEMA: &str = "\
$schema:
  name: string(non-null)
  count: number(non-null)
  flag: boolean(non-null)
  choice: enum(a, b; non-null)
  plain: string
  plain_count: number
  tags: string[](non-null)
  items: string(non-null)[]
  either: string(non-null) | number
  meta:
    note: string
    $constraints:
      non-null: true
  both: string(required; non-null)
";

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

type Problem = (ValidationProblemKind, ValidationProblemCode, String);

/// The one problem of a failed validation.
#[track_caller]
fn only_problem(result: &Result<ValidationSuccess<'_>, ValidationFailure>) -> Problem {
    let Err(failure) = result else {
        panic!("expected a failure, got {result:?}");
    };
    let [problem] = failure.problems() else {
        panic!("expected exactly one problem, got {:?}", failure.problems());
    };
    (problem.kind, problem.code, problem.path.clone())
}

#[track_caller]
fn accepted(result: Result<ValidationSuccess<'_>, ValidationFailure>) -> Value {
    match result {
        Ok(success) => success.value.into_owned(),
        Err(failure) => panic!("expected the value accepted, got {:?}", failure.problems()),
    }
}

/// The type problem an explicit `null` at `/{key}` raises.
fn null_problem(key: &str) -> Problem {
    (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch, format!("/{key}"))
}

const MARKED: &[&str] = &["name", "count", "flag", "choice", "tags", "items", "either", "meta", "both"];

#[test]
fn an_absent_marked_key_is_valid() {
    let schema = schema(SCHEMA);
    let document = json!({ "both": "x" });
    assert_eq!(accepted(schema.validate_strict(&document)), document);
    assert_eq!(accepted(schema.validate_soft(&document)), document);
}

#[test]
fn an_explicit_null_is_rejected_under_strict_and_soft_validation() {
    let schema = schema(SCHEMA);
    for key in MARKED {
        let mut document = json!({ "both": "x" });
        document[*key] = Value::Null;
        // An `enum` lowers to its members and a union to `anyOf`, neither
        // with a top-level `type`, so `null` fails as a constraint there, as
        // it does for the `required` spelling of each.
        let expected = if ["choice", "either"].contains(key) {
            (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation, format!("/{key}"))
        } else {
            null_problem(key)
        };
        assert_eq!(only_problem(&schema.validate_strict(&document)), expected, "strict {key}");
        assert_eq!(only_problem(&schema.validate_soft(&document)), expected, "soft {key}");
    }
    let required = self::schema("$schema:\n  choice: enum(a, b; required)\n  either: string(required) | number\n");
    for key in ["choice", "either"] {
        let mut document = json!({ "choice": "a", "either": "a" });
        document[key] = Value::Null;
        assert_eq!(
            only_problem(&required.validate_strict(&document)),
            (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation, format!("/{key}"))
        );
    }
}

/// Soft validation rejects the `null` itself: a convertible sibling does not
/// let it through, and it is neither converted to the text `"null"` nor
/// read as an absent key, either of which would have passed.
#[test]
fn soft_validation_rejects_the_null_before_any_conversion() {
    let schema = schema(SCHEMA);
    let document = json!({ "both": "x", "name": null, "plain_count": "4" });
    let result = schema.validate_soft(&document);
    assert_eq!(only_problem(&result), null_problem("name"));
    // The same document without the null converts its sibling, so the null
    // alone made the difference.
    let without = json!({ "both": "x", "plain_count": "4" });
    assert_eq!(accepted(schema.validate_soft(&without)), json!({ "both": "x", "plain_count": 4 }));
    // `null` for a marked string is neither the text `"null"` nor absent.
    for converted in [json!({ "both": "x", "name": "null" }), json!({ "both": "x" })] {
        assert!(schema.validate_strict(&converted).is_ok(), "{converted} is valid, so null did not become it");
    }
}

#[test]
fn the_unmarked_control_still_accepts_null() {
    let schema = schema(SCHEMA);
    for key in ["plain", "plain_count"] {
        let mut document = json!({ "both": "x" });
        document[key] = Value::Null;
        assert_eq!(accepted(schema.validate_strict(&document)), document, "strict {key}");
        assert_eq!(accepted(schema.validate_soft(&document)), document, "soft {key}");
    }
}

/// A wrong, non-null type is judged exactly as on the unmarked property.
#[test]
fn a_wrong_type_is_judged_as_on_the_unmarked_property() {
    let schema = schema(SCHEMA);
    let marked = json!({ "both": "x", "name": 5 });
    let control = json!({ "both": "x", "plain": 5 });
    assert_eq!(only_problem(&schema.validate_strict(&marked)), null_problem("name"));
    assert_eq!(only_problem(&schema.validate_strict(&control)), null_problem("plain"));
    let soft_marked = schema.validate_soft(&marked).map(|success| success.value.into_owned());
    let soft_control = schema.validate_soft(&control).map(|success| success.value.into_owned());
    match (soft_marked, soft_control) {
        (Ok(marked), Ok(control)) => assert_eq!(marked["name"], control["plain"]),
        (Err(marked), Err(control)) => {
            let shape = |failure: &ValidationFailure| -> Vec<(ValidationProblemKind, ValidationProblemCode)> {
                failure.problems().iter().map(|problem| (problem.kind, problem.code)).collect()
            };
            assert_eq!(shape(&marked), shape(&control));
        }
        (marked, control) => panic!("soft outcomes differ: {marked:?} vs {control:?}"),
    }
}

#[test]
fn empty_values_are_judged_by_the_type_never_as_null_or_absent() {
    let schema = schema(SCHEMA);
    let document = json!({ "both": "", "name": "", "tags": [], "items": [], "meta": {} });
    assert_eq!(accepted(schema.validate_strict(&document)), document);
    assert_eq!(accepted(schema.validate_soft(&document)), document);
}

#[test]
fn either_list_spelling_rejects_null_items_by_the_item_type_and_stays_optional() {
    let schema = schema(SCHEMA);
    for key in ["tags", "items"] {
        let mut document = json!({ "both": "x" });
        document[key] = json!(["a", null]);
        assert_eq!(
            only_problem(&schema.validate_strict(&document)),
            (ValidationProblemKind::Type, ValidationProblemCode::TypeMismatch, format!("/{key}/1")),
            "{key}"
        );
    }
}

#[test]
fn a_property_level_union_accepts_its_arms_and_rejects_null() {
    let schema = schema(SCHEMA);
    for value in [json!("a"), json!(4)] {
        let document = json!({ "both": "x", "either": value });
        assert_eq!(accepted(schema.validate_strict(&document)), document);
    }
}

#[test]
fn json_export_leaves_marked_properties_optional_and_never_null() {
    let exported = schema(SCHEMA).to_json_schema().expect("schema exports");
    assert_eq!(exported["required"], json!(["both"]));
    let properties = &exported["properties"];
    assert_eq!(properties["name"], json!({ "type": "string" }));
    assert_eq!(properties["count"], json!({ "type": "number" }));
    assert_eq!(properties["tags"], json!({ "type": "array", "items": { "type": "string" } }));
    assert_eq!(properties["items"], json!({ "type": "array", "items": { "type": "string" } }));
    assert_eq!(properties["either"], json!({ "anyOf": [{ "type": "string" }, { "type": "number" }] }));
    assert_eq!(properties["meta"]["type"], json!("object"));
    // The control keeps its optional null arm.
    assert_eq!(properties["plain"], json!({ "anyOf": [{ "type": "null" }, { "type": "string" }] }));
}

#[test]
fn every_type_keyword_accepts_non_null() {
    for atom in [
        "string(non-null)",
        "number(non-null)",
        "boolean(non-null)",
        "boolish(non-null)",
        "numberlike(non-null)",
        "date(non-null)",
        "datetime(non-null)",
        "time(non-null)",
        "url(non-null)",
        "email(non-null)",
        "yaml(non-null)",
        "json(non-null)",
        "expression(non-null)",
        "shell(non-null)",
        "alphanumeric(non-null)",
        "object(non-null)",
        "file(non-null)",
        "schema(non-null)",
        "type-definition(non-null)",
        "enum(a, b; non-null)",
        "literal('x'; non-null)",
        "string[](non-null)",
        "tuple([string, number]; non-null)",
        "{ a: string }(non-null)",
    ] {
        let text = format!("$schema:\n  value: \"{atom}\"\n");
        let schema = schema(&text);
        let exported = schema.to_json_schema().unwrap_or_else(|error| panic!("`{atom}` exports: {error}"));
        assert!(exported.get("required").is_none(), "`{atom}` stays optional: {exported}");
        assert!(schema.validate_strict(&json!({})).is_ok(), "`{atom}` may be absent");
        let problems = match schema.validate_strict(&json!({ "value": null })) {
            Err(failure) => failure.problems().to_vec(),
            Ok(_) => panic!("`{atom}` rejects null"),
        };
        assert!(problems.iter().all(|problem| problem.path == "/value"), "`{atom}`: {problems:?}");
    }
}

#[test]
fn a_null_admitting_union_is_a_declaration_error_at_construction() {
    let error = SimplifiedSchema::try_from("$schema:\n  value: null | string(non-null)\n")
        .expect_err("a non-null arm beside null is rejected");
    assert!(error.to_string().contains("non-null"), "{error}");
}

/// The grammar cannot see what a named type admits; resolution checks each
/// `non-null` use after expansion.
#[test]
fn a_non_null_use_of_a_null_admitting_named_type_is_a_declaration_error() {
    for definition in ["string | null", "unknown"] {
        let text = format!("$schema:\n  value: Opt(non-null)\ntypes:\n  Opt: {definition}\n");
        let error = match SimplifiedSchema::try_from(text.as_str()) {
            Err(SimplifiedSchemaError::Schema(error)) => error,
            other => panic!("expected a schema error for `{definition}`, got {other:?}"),
        };
        assert!(
            contains_grammar(&error, "$schema.value", "`non-null` cannot mark a type that admits null"),
            "`{definition}`: {error:?}"
        );
    }
    // A recursive definition stays a reference at its use, and is checked
    // through what its definition admits.
    let recursive = "$schema:\n  value: List(non-null)\ntypes:\n  List:\n    - null\n    - next: List\n";
    match SimplifiedSchema::try_from(recursive) {
        Err(SimplifiedSchemaError::Schema(error)) => assert!(
            contains_grammar(&error, "$schema.value", "`non-null` cannot mark a type that admits null"),
            "recursive: {error:?}"
        ),
        other => panic!("expected a schema error for the recursive use, got {other:?}"),
    }
    // A named type that excludes null takes the marker.
    let schema = schema("$schema:\n  value: Opt(non-null)\ntypes:\n  Opt: string | number\n");
    assert!(schema.validate_strict(&json!({})).is_ok());
    assert_eq!(
        only_problem(&schema.validate_strict(&json!({ "value": null }))),
        (ValidationProblemKind::Invalid, ValidationProblemCode::ConstraintViolation, "/value".to_string())
    );
}

/// Whether `error`, or an error it wraps, is the grammar error `message` at
/// `property`.
fn contains_grammar(error: &SchemaError, property: &str, message: &str) -> bool {
    match error {
        SchemaError::Grammar { property: actual, message: actual_message, .. } => {
            actual == property && actual_message.starts_with(message)
        }
        SchemaError::SchemaDocument { source: Some(source), .. } => contains_grammar(source, property, message),
        SchemaError::Aggregate { errors } => errors.iter().any(|error| contains_grammar(error, property, message)),
        _ => false,
    }
}

/// `non-null` is a presence marker for subtyping, exactly like `required`.
#[test]
fn subtyping_treats_non_null_as_required_is_treated() {
    let atom = |text: &str| parse_type_expr("p", text).unwrap_or_else(|error| panic!("`{text}`: {error}"));
    let def = |text: &str| -> PropertyDef {
        parse_property_definition("p", &serde_yaml_ng::Value::String(text.into()))
            .unwrap_or_else(|error| panic!("`{text}`: {error}"))
    };
    for (marked, plain) in [("string(non-null)", "string"), ("string(non-null)[]", "string[]")] {
        let required = marked.replace("non-null", "required");
        for (left, right) in [(marked, plain), (plain, marked)] {
            let (required_left, required_right) =
                if left == marked { (required.as_str(), plain) } else { (plain, required.as_str()) };
            assert_eq!(
                is_subtype(&atom(left), &atom(right)),
                is_subtype(&atom(required_left), &atom(required_right)),
                "{left} <: {right}"
            );
            assert_eq!(
                is_definition_subtype(&def(left), &def(right)),
                is_definition_subtype(&def(required_left), &def(required_right)),
                "{left} <: {right} as definitions"
            );
        }
    }
    assert!(is_subtype(&atom("string(non-null)"), &atom("string")));
    assert!(is_subtype(&atom("string"), &atom("string(non-null)")));
}
