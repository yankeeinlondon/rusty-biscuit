//! The grammar of the declaration-narrowing language additions: `non-null`,
//! `$constraints: { closed: true }`, and the `schema` / `type-definition`
//! options `mapping`, `property(name, type-definition(...))`, `forms(...)`,
//! and `declared-required`.
//!
//! Each accepted spelling parses to its `Constraint` and survives a canonical
//! serialize-and-reparse round trip; each rejected spelling is a located
//! `SchemaError::Grammar`. What the constraints *do* is tested beside their
//! lowering (`closed_roots.rs`, `non_null_properties.rs`,
//! `declaration_narrowing.rs`).

use serde_yaml_ng::Value as YamlValue;
use simplified_schema::simplified::{
    Constraint, PropertyDef, SchemaDeclaration, SchemaRoot, SimplifiedType, TypeExpr, parse_property_definition,
    parse_schema_declaration, serialize_property_def,
};
use simplified_schema::{SchemaError, SimplifiedSchema};

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

#[track_caller]
fn definition(expression: &str) -> PropertyDef {
    parse_property_definition("p", &YamlValue::String(expression.into()))
        .unwrap_or_else(|error| panic!("`{expression}` parses: {error}"))
}

#[track_caller]
fn grammar_error(value: &YamlValue) -> String {
    match parse_property_definition("p", value) {
        Err(SchemaError::Grammar { message, .. }) => message,
        other => panic!("expected a grammar error for {value:?}, got {other:?}"),
    }
}

#[track_caller]
fn expression_error(expression: &str) -> (String, std::ops::Range<usize>) {
    match parse_property_definition("p", &YamlValue::String(expression.into())) {
        Err(SchemaError::Grammar { message, span, .. }) => (message, span),
        other => panic!("expected a grammar error for `{expression}`, got {other:?}"),
    }
}

fn atom_constraints(def: &PropertyDef) -> Vec<Constraint> {
    match def {
        PropertyDef::Single(atom) => atom.property_constraints().cloned().collect(),
        PropertyDef::Union(_) => panic!("expected one atom"),
    }
}

mod accepted {
    use super::*;

    /// Every new spelling, alone and nested, re-parses from its canonical
    /// serialization to an equal definition.
    #[test]
    fn every_new_spelling_round_trips_through_canonical_serialization() {
        for expression in [
            "string(non-null)",
            "string[](non-null)",
            "string(non-null)[]",
            "number(non-null; min(1))",
            "{ a: string }(non-null)",
            "schema(mapping)",
            "schema(mapping; non-null)",
            "schema(required; property(kind, type-definition(forms(literal, enum); declared-required)))",
            "schema(property(kind, type-definition(declared-required)); property(name, type-definition(forms(string))))",
            "type-definition(forms(literal))",
            "type-definition(declared-required)",
            "type-definition(forms(literal, enum, literal); declared-required; non-null)",
            "schema(required; non-null)",
        ] {
            let parsed = definition(expression);
            let canonical = serialize_property_def(&parsed);
            let reparsed = definition(&canonical);
            assert_eq!(reparsed, parsed, "`{expression}` → `{canonical}`");
            assert_eq!(serialize_property_def(&reparsed), canonical, "canonical form is stable");
        }
    }

    #[test]
    fn non_null_is_a_marker_in_either_constraint_list() {
        assert_eq!(atom_constraints(&definition("string(non-null)")), [Constraint::NonNull]);
        assert_eq!(atom_constraints(&definition("string[](non-null)")), [Constraint::NonNull]);
        assert_eq!(atom_constraints(&definition("string(non-null)[]")), [Constraint::NonNull]);
        assert!(definition("string[](non-null)").marks_non_null());
        // Redundant beside `required`, and accepted.
        assert_eq!(
            atom_constraints(&definition("string(required; non-null)")),
            [Constraint::Required, Constraint::NonNull]
        );
    }

    #[test]
    fn the_nested_property_option_carries_a_type_definition_declaration() {
        let def = definition("schema(required; property(kind, type-definition(forms(literal, enum); declared-required)))");
        let constraints = atom_constraints(&def);
        assert_eq!(constraints[0], Constraint::Required);
        let Constraint::Property { name, declaration } = &constraints[1] else {
            panic!("expected `property`, got {:?}", constraints[1]);
        };
        assert_eq!(name, "kind");
        let PropertyDef::Single(inner) = declaration.as_ref() else { panic!("one atom") };
        assert_eq!(inner.ty, TypeExpr::Primitive(SimplifiedType::TypeDefinition));
        assert_eq!(
            inner.constraints,
            [Constraint::Forms(vec![SimplifiedType::Literal, SimplifiedType::Enum]), Constraint::DeclaredRequired]
        );
    }

    #[test]
    fn forms_is_a_set_and_property_may_repeat() {
        assert_eq!(
            atom_constraints(&definition("type-definition(forms(enum, literal, enum))")),
            [Constraint::Forms(vec![SimplifiedType::Enum, SimplifiedType::Literal])]
        );
        let repeated = atom_constraints(&definition(
            "schema(property(kind, type-definition(declared-required)); property(id, type-definition(forms(string))))",
        ));
        assert_eq!(repeated.len(), 2);
        assert!(repeated.iter().all(|constraint| matches!(constraint, Constraint::Property { .. })));
    }

    #[test]
    fn closed_is_a_dollar_constraints_flag_on_roots_arms_and_nested_objects() {
        let single = parse_schema_declaration(&yaml("a: string\n$constraints: { closed: true }\n")).unwrap();
        let SchemaDeclaration::Schema(SchemaRoot::Single(shape)) = single else { panic!("a single root") };
        assert_eq!(shape.constraints, [Constraint::Closed]);
        assert!(!shape.properties.contains_key("$constraints"));

        let union =
            parse_schema_declaration(&yaml("- a: string\n  $constraints: { closed: true }\n- b: string\n")).unwrap();
        let SchemaDeclaration::Schema(SchemaRoot::Union(arms)) = union else { panic!("a root union") };
        let closures: Vec<_> = arms
            .iter()
            .map(|arm| match arm {
                simplified_schema::simplified::SchemaArm::Inline(shape) => shape.constraints.clone(),
                other => panic!("inline arm, got {other:?}"),
            })
            .collect();
        assert_eq!(closures, [vec![Constraint::Closed], vec![]]);

        let nested = definition_of_yaml("box:\n  a: string\n  $constraints:\n    closed: true\n", "box");
        let PropertyDef::Single(atom) = nested else { panic!("one atom") };
        let TypeExpr::InlineObject(inner) = atom.ty else { panic!("inline object") };
        assert_eq!(inner.constraints, [Constraint::Closed]);
    }

    fn definition_of_yaml(text: &str, key: &str) -> PropertyDef {
        let SchemaDeclaration::Schema(SchemaRoot::Single(mut shape)) = parse_schema_declaration(&yaml(text)).unwrap()
        else {
            panic!("a single root")
        };
        shape.properties.shift_remove(key).expect("key present")
    }

    #[test]
    fn closed_and_non_null_survive_the_block_form_export_round_trip() {
        let text = "$schema:\n  a: string(non-null)\n  box:\n    b: string\n    $constraints:\n      non-null: true\n      closed: true\n  $constraints:\n    closed: true\n";
        let schema = SimplifiedSchema::try_from(text).expect("schema parses");
        let exported = schema.to_simplified_yaml().expect("exports");
        assert!(exported.contains("closed: true"), "{exported}");
        let reparsed = SimplifiedSchema::try_from(exported.as_str()).expect("export re-parses");
        assert_eq!(reparsed.to_simplified_yaml().unwrap(), exported);
        assert_eq!(reparsed.property("a").unwrap().def(), schema.property("a").unwrap().def());
        assert_eq!(reparsed.property("box").unwrap().def(), schema.property("box").unwrap().def());
        assert!(schema.property("box").unwrap().def().marks_non_null());
    }

    /// The library's definition of `kind: yaml-schema` documents parses as a
    /// schema payload.
    #[test]
    fn the_draft_yaml_schema_definition_parses_as_a_schema_payload() {
        let text = include_str!("../../../schemas/yaml-schema.yaml");
        let document = yaml(text);
        let payload = document.get("$schema").expect("the draft exports `$schema`");
        let SchemaDeclaration::Schema(SchemaRoot::Single(shape)) =
            parse_schema_declaration(payload).unwrap_or_else(|error| panic!("draft payload parses: {error}"))
        else {
            panic!("a single root")
        };
        assert_eq!(shape.constraints, [Constraint::Closed]);
        let types = &shape.properties["types"];
        assert_eq!(atom_constraints(types), [Constraint::Mapping, Constraint::NonNull]);
        let export = atom_constraints(&shape.properties["$schema"]);
        assert!(matches!(&export[1], Constraint::Property { name, .. } if name == "kind"), "{export:?}");
        for key in ["description", "categories"] {
            assert!(shape.properties[key].marks_non_null(), "{key}");
        }
    }
}

mod rejected {
    use super::*;

    #[test]
    fn declaration_options_are_rejected_off_their_meta_type() {
        for (expression, needle) in [
            ("string(mapping)", "`mapping` is an option of `schema` and is not valid on `string`"),
            ("type-definition(mapping)", "`mapping` is an option of `schema`"),
            ("schema[](mapping)", "not valid on `schema` arrays"),
            ("schema(forms(literal))", "`forms` is an option of `type-definition`"),
            ("schema(declared-required)", "`declared-required` is an option of `type-definition`"),
            ("number(declared-required)", "`declared-required` is an option of `type-definition`"),
            ("type-definition(property(kind, type-definition(declared-required)))", "`property` is an option of `schema`"),
            ("Alias(mapping)", "not valid on a named-type reference"),
        ] {
            let (message, span) = expression_error(expression);
            assert!(message.contains(needle), "`{expression}`: {message}");
            assert!(!span.is_empty(), "`{expression}` is located: {span:?}");
        }
    }

    #[test]
    fn option_arities_and_arguments_are_checked() {
        for (expression, needle) in [
            ("type-definition(forms())", "`forms` requires at least one type keyword"),
            ("type-definition(forms)", "`forms` requires arguments"),
            ("type-definition(forms(nope))", "`nope` is not one"),
            ("type-definition(forms('literal'))", "is not one"),
            ("schema(mapping(1))", "`mapping` does not take arguments"),
            ("type-definition(declared-required(x))", "`declared-required` does not take arguments"),
            ("string(non-null(1))", "`non-null` does not take arguments"),
            ("schema(property)", "`property` requires arguments"),
            ("schema(property(kind))", "takes a name and a `type-definition(...)`"),
            ("schema(property(, type-definition))", "requires a property name"),
            ("schema(property(kind, string))", "with a `type-definition(...)` expression"),
            ("schema(property(kind, type-definition[]))", "with a `type-definition(...)` expression"),
            ("schema(property(kind, type-definition(forms(string)))", "missing `)`"),
        ] {
            let (message, _) = expression_error(expression);
            assert!(message.contains(needle), "`{expression}`: {message}");
        }
    }

    #[test]
    fn non_null_over_a_null_admitting_type_is_a_declaration_error() {
        for expression in [
            "null | string(non-null)",
            "string(non-null) | null",
            "unknown(non-null)",
            "any(non-null)",
            "null(non-null)",
            "string(non-null) | unknown",
        ] {
            let (message, span) = expression_error(expression);
            assert!(message.contains("`non-null` cannot mark a type that admits null"), "`{expression}`: {message}");
            assert!(!span.is_empty(), "`{expression}`");
        }
        // The span is the marked option.
        let (_, span) = expression_error("null | string(non-null)");
        assert_eq!(&"null | string(non-null)"[span], "string(non-null)");

        // The YAML list spelling of the same union.
        let message = grammar_error(&yaml("[string(non-null), null]"));
        assert!(message.contains("`non-null` cannot mark"), "{message}");
        let message = grammar_error(&yaml("[string(non-null), ~]"));
        assert!(message.contains("`non-null` cannot mark"), "{message}");

        // A list of a nullable type is itself never null: accepted.
        definition("(null | string)[](non-null)");
        definition("null[](non-null)");
    }

    #[test]
    fn closed_must_be_the_flag_true() {
        for value in ["false", "1", "'true'", "~", "[true]"] {
            let text = format!("a: string\n$constraints:\n  closed: {value}\n");
            match parse_schema_declaration(&yaml(&text)) {
                Err(SchemaError::Grammar { message, .. }) => {
                    assert!(message.contains("`$constraints.closed` is a flag constraint and must be `true`"), "{message}");
                }
                other => panic!("`closed: {value}` must be rejected, got {other:?}"),
            }
        }
    }
}
