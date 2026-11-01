//! The `schema` / `type-definition` declaration options judged through
//! validation: `mapping`, `property(name, type-definition(...))`,
//! `forms(...)`, and `declared-required`.
//!
//! Every option judges the authored declaration a value carries, parse-only:
//! a reference carrier or reference arm is skipped and a named-type
//! alternative passes `forms` provisionally (`claimed_values.rs` covers what
//! preparation then checks). Each row runs in strict and soft mode, which
//! must agree.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure, ValidationProblemCode};

const KIND_CLAIM: &str = "schema(property(kind, type-definition(forms(literal, enum); declared-required)))";
const FORMS_MESSAGE: &str = "`kind` must be declared as `literal(...)` or `enum(...)`";
const REQUIRED_MESSAGE: &str = "`kind` must be declared `required`, with no alternative that admits null";

fn schema_with(declaration: &str) -> SimplifiedSchema {
    let text = format!("$schema:\n  x: '{declaration}'\n");
    SimplifiedSchema::try_from(text.as_str()).unwrap_or_else(|error| panic!("`{declaration}` builds: {error}"))
}

/// The value `{x: {kind: <kind>}}`, or `{x: <carrier>}` via [`with_carrier`].
fn kind_declared_as(kind: &str) -> Value {
    json!({ "x": { "kind": kind } })
}

fn with_carrier(carrier: Value) -> Value {
    json!({ "x": carrier })
}

#[track_caller]
fn assert_accepts(schema: &SimplifiedSchema, value: &Value) {
    for (mode, result) in [("strict", schema.validate_strict(value)), ("soft", schema.validate_soft(value))] {
        if let Err(failure) = result {
            panic!("{mode} validation rejected {value}: {failure:?}");
        }
    }
}

/// Asserts both modes reject `value` with exactly one constraint violation
/// at `/x` carrying `message`.
#[track_caller]
fn assert_rejects(schema: &SimplifiedSchema, value: &Value, message: &str) {
    for (mode, result) in [("strict", schema.validate_strict(value)), ("soft", schema.validate_soft(value))] {
        let Err(ValidationFailure::InvalidData { problems }) = result else {
            panic!("{mode} validation of {value}: expected invalid data, got {result:?}");
        };
        let found: Vec<_> = problems
            .iter()
            .map(|problem| (problem.path.as_str(), problem.code, problem.message.as_str()))
            .collect();
        assert_eq!(found, [("/x", ValidationProblemCode::ConstraintViolation, message)], "{mode} validation of {value}");
    }
}

mod kind_declarations {
    use super::*;

    #[test]
    fn an_absent_kind_is_rejected() {
        assert_rejects(&schema_with(KIND_CLAIM), &json!({ "x": { "name": "string" } }), "the schema must declare `kind`");
    }

    #[test]
    fn a_string_kind_is_rejected_by_forms_whether_or_not_it_is_required() {
        let schema = schema_with(KIND_CLAIM);
        assert_rejects(&schema, &kind_declared_as("string"), FORMS_MESSAGE);
        assert_rejects(&schema, &kind_declared_as("string(required)"), FORMS_MESSAGE);
    }

    /// `forms` lists constructors, but a `literal` or `enum` alternative
    /// must still denote non-empty strings.
    #[test]
    fn numeric_and_boolean_literals_are_rejected_by_forms() {
        let schema = schema_with(KIND_CLAIM);
        assert_rejects(
            &schema,
            &kind_declared_as("literal(1; required)"),
            &format!("{FORMS_MESSAGE}; `literal(1;required)` does not denote non-empty strings"),
        );
        assert_rejects(
            &schema,
            &kind_declared_as("literal(true; required)"),
            &format!("{FORMS_MESSAGE}; `literal(true;required)` does not denote non-empty strings"),
        );
    }

    /// An empty `enum()` never reaches the option: the declaration itself
    /// does not parse.
    #[test]
    fn an_empty_enum_is_a_grammar_error_inside_the_declaration() {
        let schema = schema_with(KIND_CLAIM);
        let value = kind_declared_as("enum(; required)");
        for result in [schema.validate_strict(&value), schema.validate_soft(&value)] {
            let Err(ValidationFailure::InvalidData { problems }) = result else {
                panic!("expected invalid data, got {result:?}");
            };
            assert_eq!(problems.len(), 1, "{problems:?}");
            assert_eq!(problems[0].path, "/x");
            assert!(problems[0].message.contains("enum"), "{}", problems[0].message);
            assert!(!problems[0].message.contains("must be declared"), "{}", problems[0].message);
        }
    }

    #[test]
    fn a_null_alternative_is_rejected_by_forms() {
        assert_rejects(&schema_with(KIND_CLAIM), &kind_declared_as("null | literal(a; required)"), FORMS_MESSAGE);
    }

    #[test]
    fn an_optional_literal_is_rejected_by_declared_required() {
        assert_rejects(&schema_with(KIND_CLAIM), &kind_declared_as("literal(a)"), REQUIRED_MESSAGE);
    }

    /// With `forms` alone, a null alternative made acceptable to `forms`
    /// still leaves the declaration optional.
    #[test]
    fn declared_required_rejects_a_declaration_made_optional_by_null() {
        let schema = schema_with("schema(property(kind, type-definition(forms(literal, null); declared-required)))");
        assert_rejects(&schema, &kind_declared_as("null | literal(a; required)"), REQUIRED_MESSAGE);
    }

    #[test]
    fn required_string_literals_and_enums_are_accepted() {
        let schema = schema_with(KIND_CLAIM);
        for kind in [
            "literal(a; required)",
            "enum(a, b; required)",
            "literal(a; required) | literal(b; required)",
        ] {
            assert_accepts(&schema, &kind_declared_as(kind));
        }
        assert_accepts(&schema, &json!({ "x": { "kind": ["literal(a; required)", "literal(b; required)"] } }));
    }

    /// A named-type alternative denotes what it resolves to, which the
    /// parse-only check cannot see; preparation verifies it.
    #[test]
    fn a_named_type_alternative_passes_forms_provisionally() {
        let schema = schema_with(KIND_CLAIM);
        assert_accepts(&schema, &kind_declared_as("Kind(required)"));
        assert_accepts(&schema, &kind_declared_as("Kind(required)@./kinds.yaml"));
    }
}

mod carriers {
    use super::*;

    #[test]
    fn a_root_union_judges_each_inline_arm_and_names_the_failing_one() {
        let schema = schema_with(KIND_CLAIM);
        let value = with_carrier(json!([
            { "kind": "literal(a; required)" },
            { "kind": "string(required)" },
        ]));
        assert_rejects(&schema, &value, &format!("root-union arm 1: {FORMS_MESSAGE}"));
        let missing = with_carrier(json!([{ "kind": "literal(a; required)" }, { "name": "string" }]));
        assert_rejects(&schema, &missing, "root-union arm 1: the schema must declare `kind`");
    }

    #[test]
    fn a_reference_arm_of_a_root_union_is_skipped() {
        let schema = schema_with(KIND_CLAIM);
        assert_accepts(&schema, &with_carrier(json!([{ "kind": "literal(a; required)" }, "./other.yaml"])));
    }

    #[test]
    fn a_reference_carrier_is_skipped() {
        assert_accepts(&schema_with(KIND_CLAIM), &with_carrier(json!("./other.yaml")));
    }

    #[test]
    fn mapping_rejects_a_list_and_a_reference_and_accepts_a_mapping() {
        let schema = schema_with("schema(mapping)");
        assert_rejects(
            &schema,
            &with_carrier(json!([{ "a": "string" }, { "b": "string" }])),
            "the declaration must be a mapping, not a list",
        );
        assert_rejects(&schema, &with_carrier(json!("./types.yaml")), "the declaration must be a mapping, not a reference");
        assert_accepts(&schema, &with_carrier(json!({ "a": "string", "b": "number" })));
    }

    /// `mapping` judges the carrier; without it a root union is a valid
    /// declaration, and a data schema still accepts lists.
    #[test]
    fn without_mapping_lists_stay_valid() {
        assert_accepts(&schema_with("schema"), &with_carrier(json!([{ "a": "string" }, { "b": "string" }])));
        let data = SimplifiedSchema::try_from("$schema:\n  x: string[]\n").unwrap();
        assert_accepts(&data, &with_carrier(json!(["one", "two"])));
    }

    #[test]
    fn repeated_property_options_are_all_enforced() {
        let schema = schema_with(
            "schema(property(kind, type-definition(declared-required)); property(name, type-definition(forms(string))))",
        );
        assert_accepts(&schema, &with_carrier(json!({ "kind": "string(required)", "name": "string" })));
        assert_rejects(
            &schema,
            &with_carrier(json!({ "kind": "string(required)" })),
            "the schema must declare `name`",
        );
        assert_rejects(
            &schema,
            &with_carrier(json!({ "kind": "string(required)", "name": "number" })),
            "`name` must be declared as `string(...)`",
        );
        assert_rejects(
            &schema,
            &with_carrier(json!({ "kind": "string", "name": "string" })),
            REQUIRED_MESSAGE,
        );
    }
}

mod type_definition_values {
    use super::*;

    #[test]
    fn forms_and_declared_required_judge_a_type_definition_value() {
        let schema = schema_with("type-definition(forms(literal, enum); declared-required)");
        assert_accepts(&schema, &with_carrier(json!("enum(a, b; required)")));
        assert_rejects(
            &schema,
            &with_carrier(json!("string(required)")),
            "the definition must be declared as `literal(...)` or `enum(...)`",
        );
        assert_rejects(
            &schema,
            &with_carrier(json!("literal(a)")),
            "the definition must be declared `required`, with no alternative that admits null",
        );
    }

    #[test]
    fn three_forms_are_listed_with_a_serial_comma() {
        let schema = schema_with("type-definition(forms(literal, enum, string))");
        assert_rejects(
            &schema,
            &with_carrier(json!("number")),
            "the definition must be declared as `literal(...)`, `enum(...)`, or `string(...)`",
        );
    }
}

mod controls {
    use super::*;

    #[test]
    fn bare_meta_types_accept_any_valid_declaration_and_reject_invalid_ones() {
        let schema = schema_with("schema");
        assert_accepts(&schema, &kind_declared_as("string"));
        assert_accepts(&schema, &with_carrier(json!("./other.yaml")));
        let definition = schema_with("type-definition");
        assert_accepts(&definition, &with_carrier(json!("literal(1)")));
        assert_accepts(&definition, &with_carrier(json!("string")));
        let Err(ValidationFailure::InvalidData { problems }) = definition.validate_strict(&with_carrier(json!("enum()")))
        else {
            panic!("an unknown type keyword is still rejected");
        };
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].path, "/x");
    }

    #[test]
    fn bare_meta_types_lower_to_a_true_payload() {
        let json = schema_with("schema(required)").to_json_schema().unwrap();
        assert_eq!(json["properties"]["x"]["x-darkmatter-schema"], json!(true), "{json}");
        let json = schema_with("type-definition(required)").to_json_schema().unwrap();
        assert_eq!(json["properties"]["x"]["x-darkmatter-type-definition"], json!(true), "{json}");
    }
}

mod vocabulary {
    use super::*;

    #[test]
    fn options_lower_to_an_options_object() {
        let json = schema_with(
            "schema(required; mapping; property(kind, type-definition(forms(literal, enum); declared-required)))",
        )
        .to_json_schema()
        .unwrap();
        assert_eq!(
            json["properties"]["x"]["x-darkmatter-schema"],
            json!({
                "mapping": true,
                "property": [{ "name": "kind", "declaration": { "forms": ["literal", "enum"], "declaredRequired": true } }],
            }),
            "{json}"
        );
        let json = schema_with("schema(required; property(kind, type-definition))").to_json_schema().unwrap();
        assert_eq!(
            json["properties"]["x"]["x-darkmatter-schema"],
            json!({ "property": [{ "name": "kind", "declaration": true }] }),
            "{json}"
        );
    }

    /// Soft conversion treats an options-bearing meta-type like the bare
    /// one: the carrier is a declaration, so nothing in it is converted.
    #[test]
    fn soft_validation_leaves_an_options_carrier_unconverted() {
        let schema = schema_with(KIND_CLAIM);
        let value = json!({ "x": { "kind": "literal(a; required)", "count": "number(min(1))" } });
        let accepted = schema.validate_soft(&value).expect("accepted");
        assert!(!accepted.is_converted(), "{:?}", accepted.value);
    }

    /// A raw JSON Schema author writes the payload directly; a malformed one
    /// is a preparation error naming the keyword.
    #[test]
    fn a_malformed_payload_is_a_preparation_error() {
        for (payload, expected) in [
            (json!(false), "x-darkmatter-schema must be true or an options object"),
            (json!({ "closed": true }), "x-darkmatter-schema has no option `closed`; expected `mapping`, `property`"),
            (json!({ "mapping": "yes" }), "x-darkmatter-schema option `mapping` must be a boolean"),
            (json!({ "property": {} }), "x-darkmatter-schema option `property` must be an array"),
        ] {
            let raw = json!({
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                "type": "object",
                "properties": { "x": { "x-darkmatter-schema": payload } },
            });
            let schema = SimplifiedSchema::try_from(raw.to_string().as_str()).unwrap();
            let Err(ValidationFailure::Unprepared { error }) = schema.validate_strict(&json!({ "x": {} })) else {
                panic!("payload {payload} prepares");
            };
            assert!(error.to_string().contains(expected), "{payload}: {error}");
        }
    }

    #[test]
    fn a_raw_payload_is_judged_like_a_lowered_one() {
        let raw = json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": { "x": { "x-darkmatter-type-definition": { "forms": ["literal"] } } },
        });
        let schema = SimplifiedSchema::try_from(raw.to_string().as_str()).unwrap();
        assert_accepts(&schema, &with_carrier(json!("literal(a)")));
        assert_rejects(&schema, &with_carrier(json!("enum(a)")), "the definition must be declared as `literal(...)`");
    }
}

/// The public judges a host calls with a parsed atom (trigger matching)
/// agree with the lowered keywords validation runs.
mod atom_judges {
    use super::*;
    use simplified_schema::format::{judge_schema, judge_type_definition};
    use simplified_schema::simplified::{PropertyDef, parse_property_definition};

    fn atom_constraints(expression: &str) -> Vec<simplified_schema::simplified::Constraint> {
        let def = parse_property_definition("x", &serde_yaml_ng::Value::String(expression.into())).unwrap();
        let PropertyDef::Single(atom) = def else { panic!("`{expression}` is one atom") };
        atom.constraints
    }

    #[test]
    fn type_definition_judge_matches_the_lowered_keyword() {
        let constraints = atom_constraints("type-definition(forms(literal); declared-required)");
        let schema = schema_with("type-definition(forms(literal); declared-required)");
        for (carrier, accepted) in [
            (json!("literal(a; required)"), true),
            (json!("literal(a)"), false),
            (json!("enum(a, b; required)"), false),
            (json!("not a definition("), false),
        ] {
            assert_eq!(judge_type_definition(&constraints, &carrier).is_ok(), accepted, "{carrier}");
            assert_eq!(schema.validate_strict(&with_carrier(carrier.clone())).is_ok(), accepted, "{carrier}");
        }
        assert!(judge_type_definition(&[], &json!("number")).is_ok());
    }

    #[test]
    fn schema_judge_matches_the_lowered_keyword() {
        let declaration = "schema(mapping; property(kind, type-definition(forms(literal, enum); declared-required)))";
        let constraints = atom_constraints(declaration);
        let schema = schema_with(declaration);
        for (carrier, accepted) in [
            (json!({ "kind": "literal(a; required)" }), true),
            (json!({ "kind": "string(required)" }), false),
            (json!({ "title": "string" }), false),
            (json!("./a.yaml"), false),
        ] {
            assert_eq!(judge_schema(&constraints, &carrier).is_ok(), accepted, "{carrier}");
            assert_eq!(schema.validate_strict(&with_carrier(carrier.clone())).is_ok(), accepted, "{carrier}");
        }
        let error = judge_schema(&constraints, &json!({ "title": "string" })).unwrap_err();
        assert_eq!(error, "the schema must declare `kind`");
    }
}
