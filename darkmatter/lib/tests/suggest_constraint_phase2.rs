use darkmatter::markdown::schemas::{
    parse_yaml_schema,
    resolve::resolve_schema,
    simplified::{
        Constraint, PropertyDef, SimplifiedSchema, grammar::normalize_simple_decimal,
        parse_yaml_schema_with_source, serialize_property_atom, to_json_schema,
    },
};
use serde_json::{Value, json};

fn suggestions(expression: &str) -> Vec<darkmatter::markdown::schemas::simplified::SuggestionCandidate> {
    let yaml = format!("value: {expression}\n");
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(&yaml).unwrap();
    let SimplifiedSchema::Single(shape) = parse_yaml_schema(&value).unwrap() else {
        unreachable!()
    };
    let PropertyDef::Single(atom) = &shape.properties["value"] else {
        unreachable!()
    };
    atom.constraints
        .iter()
        .find_map(|constraint| match constraint {
            Constraint::Suggest(candidates) => Some(candidates.clone()),
            _ => None,
        })
        .unwrap()
}

#[test]
fn simple_decimal_normalization_is_string_based_and_canonical() {
    for (input, expected) in [
        ("0", Some("0")),
        ("-0", Some("0")),
        ("-000.000", Some("0")),
        ("0003.5000", Some("3.5")),
        ("-0012.3400", Some("-12.34")),
        ("1e3", None),
        ("+1", None),
        (".5", None),
        ("5.", None),
        (" 1 ", None),
    ] {
        assert_eq!(normalize_simple_decimal(input).as_deref(), expected, "{input}");
    }
}

#[test]
fn target_directed_interpretation_retains_numeric_fallback_metadata() {
    let strings = suggestions("string(suggest(12, true, null))");
    assert_eq!(
        strings.iter().map(|candidate| &candidate.interpreted).collect::<Vec<_>>(),
        vec![&json!("12"), &json!("true"), &json!("null")]
    );

    let numbers = suggestions("number(suggest(0003.5000, -0.00, many, 1e3))");
    assert_eq!(numbers[0].interpreted, json!(3.5));
    assert_eq!(numbers[0].canonical_decimal.as_deref(), Some("3.5"));
    assert_eq!(numbers[1].interpreted, json!(0));
    assert_eq!(numbers[2].interpreted, json!("many"));
    assert_eq!(numbers[2].canonical_decimal, None);
    assert_eq!(numbers[3].interpreted, json!("1e3"));
}

#[test]
fn observable_json_round_trip_controls_lossless_number_interpretation() {
    let mut accepted = "9".to_string();
    let beyond = loop {
        let candidate = format!("{accepted}9");
        let interpreted = suggestions(&format!("number(suggest({candidate}))"));
        if interpreted[0].interpreted.is_string() {
            break candidate;
        }
        accepted = candidate;
        assert!(accepted.len() < 256, "expected an observable JSON number boundary");
    };

    assert!(suggestions(&format!("number(suggest({accepted}))"))[0]
        .interpreted
        .is_number());
    assert_eq!(
        suggestions(&format!("number(suggest({beyond}))"))[0].interpreted,
        Value::String(beyond)
    );
}

#[test]
fn serializer_is_deterministic_and_reparses_suggestions() {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("value: number(integer; suggest(003.500, many))[]\n").unwrap();
    let SimplifiedSchema::Single(shape) = parse_yaml_schema(&value).unwrap() else {
        unreachable!()
    };
    let PropertyDef::Single(atom) = &shape.properties["value"] else {
        unreachable!()
    };
    let serialized = serialize_property_atom(atom);
    assert_eq!(serialized, "number(integer;suggest(3.5,many))[]");
    let reparsed = darkmatter::markdown::schemas::simplified::grammar::parse_type_expr(
        "value",
        &serialized,
    )
    .unwrap();
    assert_eq!(serialize_property_atom(&reparsed), serialized);
}

#[test]
fn conversion_keeps_suggestions_non_validating_metadata() {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("value: string(min(2); suggest(a, valid))\n").unwrap();
    let schema = parse_yaml_schema(&value).unwrap();
    let converted = to_json_schema(&schema).unwrap();
    let fragment = converted["properties"]["value"]["anyOf"]
        .as_array()
        .unwrap()
        .iter()
        .find(|arm| arm["type"] == "string")
        .unwrap();
    assert_eq!(fragment["type"], json!("string"));
    assert_eq!(fragment["minLength"], json!(2));
    assert_eq!(fragment["x-darkmatter-suggest"], json!(["a", "valid"]));
}

#[test]
fn source_aware_parse_projects_multibyte_and_yaml_escape_ranges() {
    let yaml = "café: string\r\nvalue: \"string(suggest(alpha, 'blue\\u0020gray'))\"\r\n";
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
    let SimplifiedSchema::Single(shape) =
        parse_yaml_schema_with_source(&value, yaml, 17).unwrap()
    else {
        unreachable!()
    };
    let PropertyDef::Single(atom) = &shape.properties["value"] else {
        unreachable!()
    };
    let Constraint::Suggest(candidates) = atom
        .constraints
        .iter()
        .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let span = candidates[1].span.clone();
    assert_eq!(&yaml[span.start - 17..span.end - 17], "'blue\\u0020gray'");
    assert_eq!(candidates[1].decoded, "blue gray");
}

#[test]
fn import_suggestions_are_interpreted_after_exact_target_resolution() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("types.yaml"),
        "$schema:\n  count: number\n  flag: boolean\n",
    )
    .unwrap();

    let resolved = resolve_schema(
        &json!({ "value": "count(suggest(003.500, 4))@./types.yaml" }),
        dir.path(),
    )
    .unwrap();
    let SimplifiedSchema::Single(shape) = resolved.simplified.unwrap() else {
        unreachable!()
    };
    let PropertyDef::Single(atom) = &shape.properties["value"] else {
        unreachable!()
    };
    let Constraint::Suggest(candidates) = atom
        .constraints
        .iter()
        .find(|constraint| matches!(constraint, Constraint::Suggest(_)))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(candidates[0].interpreted, json!(3.5));
    assert_eq!(candidates[0].canonical_decimal.as_deref(), Some("3.5"));

    let unsupported = resolve_schema(
        &json!({ "value": "flag(suggest(true))@./types.yaml" }),
        dir.path(),
    )
    .unwrap_err();
    assert!(unsupported.to_string().contains("exact `string` or `number`"));
}
