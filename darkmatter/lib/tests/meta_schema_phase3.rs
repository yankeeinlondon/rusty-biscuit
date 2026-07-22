//! Phase 3 contracts for the SimplifiedSchema semantic meta-type vocabulary.

use darkmatter::markdown::{
    Markdown,
    schemas::{
        DetectOptions, PropertyDef, SimplifiedSchema, SimplifiedType, TypeExpr,
        detect_schema, schema_to_yaml, schema_type_descriptors,
        simplified::{grammar::parse_type_expr, serialize_property_atom},
    },
};

fn parse(input: &str) -> darkmatter::markdown::schemas::PropertyAtom {
    parse_type_expr("candidate", input).unwrap_or_else(|error| {
        panic!("expected {input:?} to parse, got {error}")
    })
}

fn assert_rejected(input: &str) {
    let Err(error) = parse_type_expr("candidate", input) else {
        panic!("expected {input:?} to be rejected")
    };
    assert!(
        error.to_string().contains("not valid on"),
        "unexpected error for {input:?}: {error}"
    );
}

#[test]
fn semantic_type_keywords_round_trip_canonically() {
    for keyword in ["type-definition", "schema", "type-definition[]", "schema[]"] {
        let atom = parse(keyword);
        assert_eq!(serialize_property_atom(&atom), keyword);
    }
}

#[test]
fn semantic_keyword_names_remain_valid_import_names() {
    for (source, expected_name, expected_reference, expected_array) in [
        ("schema@./schema.yaml", "schema", "./schema.yaml", false),
        ("type-definition@this", "type-definition", "this", false),
        ("schema[]@./schema.yaml", "schema", "./schema.yaml", true),
        ("type-definition[](required)@this", "type-definition", "this", true),
    ] {
        let atom = parse(source);
        assert_eq!(atom.is_array, expected_array, "{source}");
        assert!(
            matches!(
                &atom.ty,
                TypeExpr::Imported { name, reference }
                    if name == expected_name && reference == expected_reference
            ),
            "{source} must remain an import: {atom:?}"
        );
        assert_eq!(serialize_property_atom(&atom), source);
    }
}

#[test]
fn semantic_types_accept_only_definition_level_constraints() {
    for semantic_type in ["type-definition", "schema"] {
        for suffix in ["(required)", "(generated)", "(required; generated)"] {
            parse(&format!("{semantic_type}{suffix}"));
        }
        parse(&format!("{semantic_type}[](min(1); max(2); unique)"));

        for invalid in [
            "min(1)",
            "max(2)",
            "pattern(x)",
            "suggest(x)",
            "eager",
            "match(src/**)",
            "scheme(https)",
            "integer",
            "not-empty",
            "unique",
            "min-keys(1)",
            "max-keys(2)",
            "example(./example.yaml)",
        ] {
            assert_rejected(&format!("{semantic_type}({invalid})"));
        }
        for invalid in ["pattern(x)", "eager", "match(src/**)", "scheme(https)"] {
            assert_rejected(&format!("{semantic_type}[]({invalid})"));
        }
    }
}

#[test]
fn semantic_defaults_use_the_passive_parser_authority() {
    for valid in [
        "type-definition(default(string))",
        "type-definition(default('string(required)'))",
        "schema(default(./schemas/does-not-exist.yaml))",
        "schema(default('does-not-exist.yaml'))",
    ] {
        parse(valid);
    }

    for invalid in [
        "type-definition(default(not-a-type))",
        "type-definition(default(true))",
        "schema(default(true))",
        "schema(default('https://example.com/schema.yaml'))",
        "schema(default('http://example.com/schema.yaml'))",
    ] {
        assert_rejected(invalid);
    }
}

#[test]
fn semantic_type_descriptors_are_authoritative() {
    for keyword in ["type-definition", "schema"] {
        let descriptor = schema_type_descriptors()
            .iter()
            .find(|descriptor| descriptor.keyword == keyword)
            .unwrap_or_else(|| panic!("missing descriptor for {keyword}"));
        assert_eq!(descriptor.accepted_constraints, "default, required, generated");
        let description = descriptor.description.to_ascii_lowercase();
        for required in ["string", "mapping", "sequence", "parse-only", "dmls"] {
            assert!(
                description.contains(required),
                "{keyword} descriptor must mention {required:?}: {}",
                descriptor.description
            );
        }
    }
}

#[test]
fn schema_detection_preserves_carrier_only_inference() {
    let markdown = Markdown::from(concat!(
        "---\n",
        "plain_definition: string(required)\n",
        "quoted_definition: 'string(required)'\n",
        "native_schema:\n  title: string(required)\n",
        "definition_sequence: [string, number]\n",
        "---\nBody\n",
    ));
    let detected = detect_schema(&[&markdown], DetectOptions::default());
    let SimplifiedSchema::Single(shape) = &detected else {
        panic!("one document must detect as one schema shape")
    };

    for property in ["plain_definition", "quoted_definition"] {
        let PropertyDef::Single(atom) = &shape.properties[property] else {
            panic!("{property} must be one detected atom")
        };
        assert_eq!(atom.ty, TypeExpr::Primitive(SimplifiedType::String));
        assert!(!atom.is_array);
    }

    let PropertyDef::Single(native_schema) = &shape.properties["native_schema"] else {
        panic!("native mapping must be one detected atom")
    };
    assert_eq!(native_schema.ty, TypeExpr::Primitive(SimplifiedType::Object));

    let PropertyDef::Single(sequence) = &shape.properties["definition_sequence"] else {
        panic!("sequence must be one detected atom")
    };
    assert_eq!(sequence.ty, TypeExpr::Primitive(SimplifiedType::String));
    assert!(sequence.is_array);

    let serialized = schema_to_yaml(&detected);
    assert!(!serialized.contains(": type-definition"));
    assert!(!serialized.contains(": schema\n"));
}
