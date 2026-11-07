//! The container contract: `narrowing::container_admission` /
//! `denotes_container` over parsed definitions, and the
//! `type-definition(container)` declaration option that judges a
//! definition value through validation.
//!
//! The rows are the container tables of the `load_data` contract: arrays,
//! tuples, objects, and pattern dictionaries are containers, `null` arms are
//! dropped but at least one container arm is required, and an unresolved
//! named-type reference is admitted only provisionally.

use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use simplified_schema::simplified::narrowing::{ContainerAdmission, container_admission, denotes_container};
use simplified_schema::simplified::{Constraint, PropertyDef, parse_property_definition, serialize_property_def};
use simplified_schema::{SchemaError, SimplifiedSchema, ValidationFailure};

/// Definitions that denote a container.
const CONTAINERS: &[&str] = &[
    "string[]",
    "{ name: string, port: number }[]",
    "tuple([string, number])",
    "{ name: string, tags: string[] }",
    "{ <string>: number }",
    "string[] | null",
    "null | string[]",
    "unknown[]",
    "object",
    "{a: string}[] | {b: number}[]",
    "string[] | {a: string}",
    "(string | number)[]",
];

/// Definitions that do not.
const NOT_CONTAINERS: &[&str] = &[
    "string",
    "number",
    "enum(a, b)",
    "literal(a)",
    "boolean",
    "unknown",
    "any",
    "null",
    "string | number",
    "string[] | string",
    "Rows@./rows.yaml | string",
];

/// Unresolved named-type references, alone or beside containers.
const PROVISIONAL: &[&str] = &["Rows@./rows.yaml", "Rows@./rows.yaml | null", "Rows@this | string[]", "Rows"];

#[track_caller]
fn definition(expression: &str) -> PropertyDef {
    parse_property_definition("as", &YamlValue::String(expression.into()))
        .unwrap_or_else(|error| panic!("`{expression}` parses: {error}"))
}

fn container_schema() -> SimplifiedSchema {
    SimplifiedSchema::try_from("$schema:\n  x: type-definition(container)\n").expect("container schema builds")
}

#[track_caller]
fn assert_accepted(schema: &SimplifiedSchema, carrier: Value) {
    let value = json!({ "x": carrier });
    for (mode, result) in [("strict", schema.validate_strict(&value)), ("soft", schema.validate_soft(&value))] {
        if let Err(failure) = result {
            panic!("{mode} validation rejected {value}: {failure:?}");
        }
    }
}

/// The one problem both modes report for `carrier`.
#[track_caller]
fn rejection(schema: &SimplifiedSchema, carrier: Value) -> String {
    let value = json!({ "x": carrier });
    let mut messages = Vec::new();
    for (mode, result) in [("strict", schema.validate_strict(&value)), ("soft", schema.validate_soft(&value))] {
        let Err(ValidationFailure::InvalidData { problems }) = result else {
            panic!("{mode} validation of {value}: expected invalid data, got {result:?}");
        };
        assert_eq!(problems.len(), 1, "{mode} validation of {value}: {problems:?}");
        assert_eq!(problems[0].path, "/x", "{mode}");
        messages.push(problems[0].message.clone());
    }
    assert_eq!(messages[0], messages[1], "strict and soft agree");
    messages.remove(0)
}

mod predicate {
    use super::*;

    #[test]
    fn every_container_row_denotes_a_container() {
        for expression in CONTAINERS {
            let def = definition(expression);
            assert!(denotes_container(&def), "`{expression}`");
            assert_eq!(container_admission(&def), ContainerAdmission::Container, "`{expression}`");
        }
    }

    #[test]
    fn scalars_unknown_and_null_alone_are_not_containers() {
        for expression in NOT_CONTAINERS {
            let def = definition(expression);
            assert!(!denotes_container(&def), "`{expression}`");
            assert_eq!(container_admission(&def), ContainerAdmission::NotContainer, "`{expression}`");
        }
    }

    /// A bracket list is the YAML list spelling of a union of its items, not
    /// a tuple; as one string it is not a definition at all.
    #[test]
    fn a_bracket_list_of_scalars_is_a_union_not_a_tuple() {
        let def = parse_property_definition("as", &serde_yaml_ng::from_str("[string, number]").unwrap()).unwrap();
        assert!(matches!(def, PropertyDef::Union(_)), "a union: {def:?}");
        assert!(!denotes_container(&def));
        assert!(parse_property_definition("as", &YamlValue::String("[string, number]".into())).is_err());
        let containers = parse_property_definition("as", &serde_yaml_ng::from_str("['string[]', null]").unwrap()).unwrap();
        assert!(denotes_container(&containers));
    }

    #[test]
    fn an_unresolved_reference_is_provisional_never_proof() {
        for expression in PROVISIONAL {
            let def = definition(expression);
            assert_eq!(container_admission(&def), ContainerAdmission::Provisional, "`{expression}`");
            assert!(!denotes_container(&def), "`{expression}` is not known to be a container");
        }
        // An array of a reference is an array whatever the reference denotes.
        assert!(denotes_container(&definition("Rows[]@./rows.yaml")));
    }
}

mod declaration_option {
    use super::*;

    #[test]
    fn container_values_pass_the_format_check() {
        let schema = container_schema();
        for expression in CONTAINERS {
            assert_accepted(&schema, json!(expression));
        }
        assert_accepted(&schema, json!(["string[]", null]));
    }

    #[test]
    fn non_container_values_fail_the_format_check() {
        let schema = container_schema();
        for expression in NOT_CONTAINERS {
            let message = rejection(&schema, json!(expression));
            assert!(message.contains("the definition must denote a container"), "`{expression}`: {message}");
        }
        let message = rejection(&schema, json!("string"));
        assert!(message.ends_with("`string` does not"), "{message}");
        let message = rejection(&schema, json!(["string", "number"]));
        assert!(message.contains("must denote a container"), "{message}");
        // The bracket text in one string is not a definition, so it fails too.
        rejection(&schema, json!("[string, number]"));
    }

    /// Judged on the text alone: the referenced file does not exist and is
    /// never opened.
    #[test]
    fn a_named_reference_passes_provisionally_without_io() {
        let schema = container_schema();
        for expression in PROVISIONAL {
            assert_accepted(&schema, json!(expression));
        }
    }

    #[test]
    fn container_combines_with_the_other_options() {
        let schema = SimplifiedSchema::try_from("$schema:\n  x: type-definition(container; declared-required)\n").unwrap();
        assert_accepted(&schema, json!("string[](required)"));
        let message = rejection(&schema, json!("string[]"));
        assert!(message.contains("must be declared `required`"), "{message}");
        let message = rejection(&schema, json!("string(required)"));
        assert!(message.contains("must denote a container"), "{message}");
    }

    #[test]
    fn a_property_option_may_require_a_container_declaration() {
        let schema =
            SimplifiedSchema::try_from("$schema:\n  x: schema(property(rows, type-definition(container)))\n").unwrap();
        assert_accepted(&schema, json!({ "rows": "{ a: string }[]" }));
        let message = rejection(&schema, json!({ "rows": "string" }));
        assert!(message.contains("must denote a container"), "{message}");
    }
}

mod grammar {
    use super::*;

    #[track_caller]
    fn grammar_error(expression: &str) -> String {
        match parse_property_definition("p", &YamlValue::String(expression.into())) {
            Err(SchemaError::Grammar { message, span, .. }) => {
                assert!(!span.is_empty(), "`{expression}` is located");
                message
            }
            other => panic!("expected a grammar error for `{expression}`, got {other:?}"),
        }
    }

    #[test]
    fn container_parses_and_round_trips() {
        for expression in ["type-definition(container)", "type-definition(forms(object); container; declared-required)"] {
            let parsed = definition(expression);
            let canonical = serialize_property_def(&parsed);
            assert_eq!(definition(&canonical), parsed, "`{expression}` → `{canonical}`");
        }
        let PropertyDef::Single(atom) = definition("type-definition(container)") else { panic!("one atom") };
        assert_eq!(atom.constraints, [Constraint::Container]);
    }

    #[test]
    fn container_takes_no_arguments() {
        let message = grammar_error("type-definition(container(x))");
        assert!(message.contains("`container` does not take arguments"), "{message}");
    }

    #[test]
    fn container_is_only_an_option_of_type_definition() {
        for (expression, needle) in [
            ("string(container)", "`container` is an option of `type-definition` and is not valid on `string`"),
            ("schema(container)", "`container` is an option of `type-definition`"),
            ("type-definition[](container)", "not valid on `type-definition` arrays"),
        ] {
            let message = grammar_error(expression);
            assert!(message.contains(needle), "`{expression}`: {message}");
        }
    }
}

mod payload {
    use super::*;

    fn raw_schema(payload: Value) -> SimplifiedSchema {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": { "x": { "x-darkmatter-type-definition": payload } },
        });
        SimplifiedSchema::try_from(raw.to_string().as_str()).expect("raw schema builds")
    }

    #[test]
    fn container_lowers_to_a_true_flag() {
        let json = container_schema().to_json_schema().unwrap();
        let payloads: Vec<&Value> =
            json["properties"]["x"]["anyOf"].as_array().unwrap().iter().filter_map(|arm| arm.get("x-darkmatter-type-definition")).collect();
        assert_eq!(payloads, [&json!({ "container": true })], "{json}");
    }

    #[test]
    fn an_absent_or_false_flag_leaves_the_check_off() {
        for payload in [json!(true), json!({}), json!({ "container": false })] {
            let schema = raw_schema(payload.clone());
            assert_accepted(&schema, json!("string"));
        }
    }

    #[test]
    fn a_true_flag_turns_the_check_on() {
        let schema = raw_schema(json!({ "container": true }));
        assert_accepted(&schema, json!("string[]"));
        let message = rejection(&schema, json!("string"));
        assert!(message.contains("must denote a container"), "{message}");
    }

    #[test]
    fn a_null_or_non_boolean_flag_is_a_preparation_error() {
        for payload in [json!({ "container": null }), json!({ "container": "yes" }), json!({ "container": 1 })] {
            let schema = raw_schema(payload.clone());
            let Err(ValidationFailure::Unprepared { error }) = schema.validate_strict(&json!({ "x": "string[]" })) else {
                panic!("payload {payload} prepares");
            };
            assert!(
                error.to_string().contains("x-darkmatter-type-definition option `container` must be a boolean"),
                "{payload}: {error}"
            );
        }
    }
}
