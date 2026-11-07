//! `SetterValue` through its public API: reading a value's form, binding it
//! through a declared type, and the input robustness matrix for its two
//! container loaders (JSON5 and YAML flow).
//!
//! Default outcome: an inadmissible container is a typed failure. Exceptions,
//! each tested here: rejected syntax is text; empty input is `""`; `null` stays
//! null for the schema to judge; scalar spelling is kept until bound.

use serde_json::{Value, json};
use simplified_schema::coercion::{Additional, NumberShape, ObjectShape, PropertySlot, Shape};
use simplified_schema::numeric::{NumericAdmissionError, admit_json5_text_numbers};
use simplified_schema::{SetterValue, SetterValueError};

fn parse(raw: &str) -> SetterValue {
    SetterValue::parse(raw).unwrap_or_else(|error| panic!("{raw}: {error}"))
}

fn text(raw: &str) -> SetterValue {
    SetterValue::Text { raw: raw.to_string() }
}

fn container(value: Value) -> SetterValue {
    SetterValue::Container(value)
}

fn numberlike() -> Shape {
    Shape::Number(NumberShape {
        as_sent: true,
        ..NumberShape::default()
    })
}

fn boolish() -> Shape {
    Shape::Boolean { as_sent: true }
}

fn closed_object(properties: Vec<(&str, Shape)>) -> Shape {
    Shape::Object(ObjectShape {
        properties: properties
            .into_iter()
            .map(|(name, shape)| PropertySlot {
                name: name.to_string(),
                shape,
                required: false,
            })
            .collect(),
        additional: Additional::Forbidden,
        ..ObjectShape::opaque()
    })
}

fn rejected(raw: &str) -> SetterValueError {
    SetterValue::parse(raw).expect_err(raw)
}

// ── form ────────────────────────────────────────────────────────────────────

#[test]
fn json5_and_yaml_flow_both_read_as_objects() {
    assert_eq!(parse("{a:1}"), container(json!({ "a": 1 })));
    assert_eq!(parse("{ max-width: 50 }"), container(json!({ "max-width": 50 })));
    assert_eq!(parse(r#"{name:"Ann",email:"a@x"}"#), container(json!({ "name": "Ann", "email": "a@x" })));
    assert_eq!(parse("[1, 'two']"), container(json!([1, "two"])));
}

#[test]
fn json5_goes_first_so_a_mixed_value_reads_a_colon_key_through_yaml() {
    assert_eq!(parse("{ a:1, max-width: 2 }"), container(json!({ "a:1": null, "max-width": 2 })));
}

#[test]
fn both_loaders_preserve_mapping_order() {
    let SetterValue::Container(from_json5) = parse("{z: 1, a: 2, m: 3}") else {
        panic!("a container");
    };
    let SetterValue::Container(from_yaml) = parse("{ z-z: 1, a: 2, m: 3 }") else {
        panic!("a container");
    };
    let keys = |value: &Value| value.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&from_json5), ["z", "a", "m"]);
    assert_eq!(keys(&from_yaml), ["z-z", "a", "m"]);
}

#[test]
fn a_quoted_string_decodes_once() {
    assert_eq!(parse(r#""Alice""#), SetterValue::Quoted("Alice".to_string()));
    assert_eq!(parse("'Alice'"), SetterValue::Quoted("Alice".to_string()));
    assert_eq!(parse(r#""\"inner\"""#), SetterValue::Quoted(r#""inner""#.to_string()));
    assert_eq!(parse(r#""5""#).bind(None), json!("5"));
}

#[test]
fn null_is_null_and_its_lookalikes_are_not() {
    assert_eq!(parse("null"), SetterValue::Null);
    assert_eq!(parse(" null "), SetterValue::Null);
    assert_eq!(parse("Null"), text("Null"));
    assert_eq!(parse("~"), text("~"));
    assert_eq!(parse("NaN"), text("NaN"));
    assert_eq!(parse("Infinity"), text("Infinity"));
    assert_eq!(parse("-Infinity"), text("-Infinity"));
    assert_eq!(parse("1e400"), text("1e400"));
}

#[test]
fn a_scalar_keeps_its_spelling_until_bound() {
    for raw in ["1.10", "5", "true", "0x10", "+1", "Alice", "a b", "1.2.3", "2026-11-06"] {
        assert_eq!(parse(raw), text(raw), "{raw}");
    }
}

#[test]
fn empty_input_is_the_empty_string() {
    assert_eq!(parse(""), text(""));
    assert_eq!(parse("").bind(None), json!(""));
    assert_eq!(parse("").bind(Some(&Shape::number())), json!(""));
}

#[test]
fn text_that_parses_as_neither_is_text() {
    for raw in ["{open", "{a: [1}", "[1, 2", "}", "{'open: 1}", "{a: 1}}", "[a, b]", "{a: 1,, b: 2}x"] {
        assert_eq!(parse(raw), text(raw), "{raw}");
        assert_eq!(parse(raw).bind(None), json!(raw), "{raw}");
    }
}

#[test]
fn a_template_is_never_read_as_a_mapping() {
    for raw in ["{{ x }}", "{{x}}", "{{ a }} and {{ b }}", "{ a: {{ x }} }", "{{ x }}: 1"] {
        assert_eq!(parse(raw), text(raw), "{raw}");
    }
    assert_eq!(parse("{{ x }}").bind(None), json!("{{ x }}"));
    assert_eq!(parse("{{ x }}").bind(Some(&Shape::text())), json!("{{ x }}"));
}

#[test]
fn a_tagged_yaml_value_is_text_and_runs_nothing() {
    for raw in [
        "{ a: !!str 1, max-width: 2 }",
        "{ a: !custom x, max-width: 2 }",
        "{ !!str a: 1, max-width: 2 }",
        "{ \"a\":!!str 1, max-width: 2 }",
        "{ a: [ !!int 1 ], max-width: 2 }",
    ] {
        assert_eq!(parse(raw), text(raw), "{raw}");
    }
}

#[test]
fn an_exclamation_mark_inside_a_scalar_is_no_tag() {
    assert_eq!(parse("{ a: b!c, max-width: 2 }"), container(json!({ "a": "b!c", "max-width": 2 })));
    assert_eq!(parse("{ a: \"!!str\", max-width: 2 }"), container(json!({ "a": "!!str", "max-width": 2 })));
}

#[test]
fn an_anchor_or_alias_is_text() {
    for raw in ["{ a: &x 1, b: *x, max-width: 2 }", "{ a: *x, max-width: 2 }"] {
        assert_eq!(parse(raw), text(raw), "{raw}");
    }
}

#[test]
fn a_block_or_second_document_is_text() {
    for raw in [
        "{ max-width: 1 }: x",
        "{ max-width: 1 }\nb: 2\n",
        "{ max-width: 1 }\n---\nb: 2\n",
        "{ max-width: 1 } trailing",
        "{ max-width: 1 } { b: 2 }",
    ] {
        assert_eq!(parse(raw), text(raw), "{raw}");
    }
}

#[test]
fn what_yaml_flow_syntax_allows_is_accepted() {
    assert_eq!(parse("{ max-width: 1 } # note"), container(json!({ "max-width": 1 })));
    assert_eq!(parse("  { max-width: 1 }\n"), container(json!({ "max-width": 1 })));
    assert_eq!(parse("{\n  max-width: 1, # note\n  b: it's\n}"), container(json!({ "max-width": 1, "b": "it's" })));
    assert_eq!(parse("{ max-width: 'a}b', c: \"d}e\" }"), container(json!({ "max-width": "a}b", "c": "d}e" })));
    assert_eq!(parse("{ max-width: 1, tags: [a, { b: c }] }"), container(json!({ "max-width": 1, "tags": ["a", { "b": "c" }] })));
    assert_eq!(parse("{ max-width: 'it''s' }"), container(json!({ "max-width": "it's" })));
}

#[test]
fn a_scalar_yaml_key_is_text_the_way_a_path_names_it() {
    assert_eq!(parse("{ 1: a, max-width: 2 }"), container(json!({ "1": "a", "max-width": 2 })));
}

#[test]
fn json5_comments_and_trailing_commas_are_accepted() {
    assert_eq!(parse("{a: 1, /* note */ b: [2,],}"), container(json!({ "a": 1, "b": [2] })));
}

// ── container admission ─────────────────────────────────────────────────────

#[test]
fn a_duplicate_key_fails_in_both_loaders() {
    for raw in ["{a:1,a:2}", "{a:1,\"a\":2}", "[{x: {a: 1, a: 2}}]", "{ max-width: 1, max-width: 2 }", "{ b-c: { a: 1, a: 2 } }"] {
        assert!(matches!(rejected(raw), SetterValueError::DuplicateKey { .. }), "{raw}");
    }
    let SetterValueError::DuplicateKey { key } = rejected("{a:1,a:2}") else {
        panic!("a duplicate");
    };
    assert_eq!(key, "a");
    let SetterValueError::DuplicateKey { key } = rejected("{ max-width: 1, max-width: 2 }") else {
        panic!("a duplicate");
    };
    assert_eq!(key, "max-width");
}

#[test]
fn an_inadmissible_number_in_a_container_fails_in_both_loaders() {
    for (raw, reason) in [
        ("{a:NaN}", "not finite"),
        ("{a:Infinity}", "not finite"),
        ("{a:-Infinity}", "not finite"),
        ("{a:1e400}", "not finite"),
        ("{a:9007199254740993}", "cannot be represented exactly"),
        ("{a:99999999999999999999}", "cannot be represented exactly"),
        ("{a:-99999999999999999999}", "cannot be represented exactly"),
        ("[99999999999999999999]", "cannot be represented exactly"),
        ("{a:0x20000000000001}", "cannot be represented exactly"),
        ("{a:1e-400}", "would become 0"),
        ("[NaN]", "not finite"),
        ("[1, [9007199254740993]]", "cannot be represented exactly"),
        ("{ a: .nan, max-width: 1 }", "not finite"),
        ("{ a: .inf, max-width: 1 }", "not finite"),
        ("{ a: 9007199254740993, max-width: 1 }", "cannot be represented exactly"),
        ("{ a: 99999999999999999999, max-width: 1 }", "cannot be represented exactly"),
        ("{ a: 1e-400, max-width: 1 }", "would become 0"),
        ("{ a: [.inf], max-width: 1 }", "not finite"),
    ] {
        let error = rejected(raw);
        assert!(matches!(error, SetterValueError::Number { .. }), "{raw}: {error}");
        assert!(error.to_string().contains(reason), "{raw}: {error}");
    }
}

#[test]
fn a_shadowed_number_is_still_judged() {
    assert!(matches!(rejected("{a:NaN,b:1}"), SetterValueError::Number { .. }));
    assert!(rejected("{a:9007199254740993,a:1}").to_string().contains("duplicate"));
}

#[test]
fn spellings_yaml_reads_as_strings_are_never_rounded() {
    // YAML's non-finite spelling is `.nan`, and the YAML loader keeps `1e400` as
    // the string it wrote, in frontmatter too. A declared number still rejects it.
    assert_eq!(parse("{ a: NaN, max-width: 1 }"), container(json!({ "a": "NaN", "max-width": 1 })));
    let overflow = parse("{ a: 1e400, max-width: 1 }");
    assert_eq!(overflow, container(json!({ "a": "1e400", "max-width": 1 })));
    let declared = closed_object(vec![("a", Shape::number()), ("max-width", Shape::number())]);
    assert_eq!(overflow.bind(Some(&declared)), json!({ "a": "1e400", "max-width": 1 }));
}

#[test]
fn a_bare_inadmissible_number_stays_text() {
    for raw in ["NaN", "Infinity", "9007199254740993", "99999999999999999999", "-99999999999999999999", "1e400", "1e-400"] {
        assert_eq!(parse(raw), text(raw), "{raw}");
        assert_eq!(parse(raw).bind(None), json!(raw), "{raw}");
    }
}

#[test]
fn json5_number_admission_reports_where() {
    let error = admit_json5_text_numbers("{\n  a: [1, 9007199254740993]\n}").unwrap_err();
    assert_eq!(error.pointer, "/a/1");
    assert_eq!(error.spelling, "9007199254740993");
    assert_eq!(error.reason, NumericAdmissionError::Inexact);
    assert_eq!((error.line, error.column), (2, 10));
}

#[test]
fn json5_number_admission_accepts_every_admissible_spelling() {
    for raw in ["1", "-1", "+1", "1.10", ".5", "5.", "1e3", "0x10", "-0xFF", "9007199254740992", "{a: 0.1}"] {
        assert_eq!(admit_json5_text_numbers(raw), Ok(()), "{raw}");
    }
    assert_eq!(admit_json5_text_numbers("not json5 'at all"), Ok(()));
    let beyond_i128 = format!("[0x{}]", "F".repeat(40));
    assert_eq!(admit_json5_text_numbers(&beyond_i128).unwrap_err().reason, NumericAdmissionError::Inexact);
}

// ── binding ─────────────────────────────────────────────────────────────────

#[test]
fn text_goes_through_the_declared_type() {
    let scalars = Shape::Union(vec![Shape::text(), Shape::number()]);
    for (raw, shape, expected) in [
        ("1.10", Shape::text(), json!("1.10")),
        ("1.10", scalars.clone(), json!("1.10")),
        ("5", Shape::number(), json!(5)),
        ("1.10", Shape::number(), json!(1.1)),
        ("5", Shape::text(), json!("5")),
        ("true", Shape::text(), json!("true")),
        ("true", Shape::boolean(), json!(true)),
        ("yes", Shape::boolean(), json!(true)),
        ("02134", numberlike(), json!("02134")),
        ("On", boolish(), json!("On")),
    ] {
        assert_eq!(parse(raw).bind(Some(&shape)), expected, "{raw} under {shape:?}");
    }
}

#[test]
fn a_value_the_type_rejects_stays_as_written_for_validation_to_report() {
    for (raw, shape) in [
        ("abc", Shape::number()),
        ("abc", Shape::boolean()),
        ("abc", closed_object(vec![])),
        ("abc", Shape::array(Shape::text())),
        ("9007199254740993", Shape::number()),
        ("1e400", Shape::number()),
        ("0x10", Shape::number()),
    ] {
        assert_eq!(parse(raw).bind(Some(&shape)), json!(raw), "{raw} under {shape:?}");
    }
}

#[test]
fn with_no_declaration_text_is_guessed_as_today() {
    for (raw, expected) in [
        ("5", json!(5)),
        ("1.10", json!(1.1)),
        ("true", json!(true)),
        ("false", json!(false)),
        ("0x10", json!(16)),
        ("Alice", json!("Alice")),
    ] {
        assert_eq!(parse(raw).bind(None), expected, "{raw}");
    }
}

#[test]
fn a_type_declaring_no_structure_guesses_too() {
    assert_eq!(parse("5").bind(Some(&Shape::Any)), json!(5));
    assert_eq!(parse("5").bind(Some(&Shape::Deferrable(Box::new(Shape::Any)))), json!(5));
    assert_eq!(parse("5").bind(Some(&Shape::Deferrable(Box::new(Shape::text())))), json!("5"));
}

#[test]
fn a_quoted_string_is_not_converted_by_binding() {
    assert_eq!(parse(r#""5""#).bind(Some(&Shape::number())), json!("5"));
    assert_eq!(parse(r#""5""#).bind(Some(&Shape::text())), json!("5"));
}

#[test]
fn null_binds_to_null_whatever_the_type() {
    for shape in [Shape::text(), Shape::number(), Shape::Null, Shape::Any] {
        assert_eq!(parse("null").bind(Some(&shape)), Value::Null, "{shape:?}");
    }
    assert_eq!(parse("null").bind(None), Value::Null);
}

// ── robustness matrix: fields inside a parsed container ─────────────────────

fn author() -> Shape {
    closed_object(vec![
        ("name", Shape::text()),
        ("age", Shape::number()),
        ("tags", Shape::array(Shape::number())),
    ])
}

/// Each row runs once per loader; a `max-width` key is what forces YAML.
fn both_loaders(body: &str, expected: Value) {
    let json5 = format!("{{{body}}}");
    let yaml = format!("{{ only-yaml: 1, {body} }}");
    let bound = parse(&json5).bind(Some(&author()));
    assert_eq!(bound, expected, "JSON5 {json5}");

    let mut with_extra = expected;
    let extended = with_extra.as_object_mut().unwrap();
    let mut reordered = serde_json::Map::new();
    reordered.insert("only-yaml".to_string(), json!(1));
    reordered.append(extended);
    assert_eq!(parse(&yaml).bind(None), Value::Object(reordered), "YAML {yaml}");
}

#[test]
fn control_a_conversion_inside_a_container_converts() {
    let bound = parse(r#"{name: 1.5, age: "5", tags: ["1", 2]}"#).bind(Some(&author()));
    assert_eq!(bound, json!({ "name": "1.5", "age": 5, "tags": [1, 2] }));
    let bound = parse(r#"{ name: 1.5, age: "5", tags: ["1", 2] }"#).bind(Some(&author()));
    assert_eq!(bound, json!({ "name": "1.5", "age": 5, "tags": [1, 2] }));
    // The same fields, forced through the YAML loader.
    let forced = closed_object(vec![
        ("max-width", Shape::number()),
        ("age", Shape::number()),
    ]);
    assert_eq!(parse(r#"{ max-width: "2", age: "5" }"#).bind(Some(&forced)), json!({ "max-width": 2, "age": 5 }));
}

#[test]
fn an_absent_field_stays_absent() {
    both_loaders("name: 'Ann'", json!({ "name": "Ann" }));
}

#[test]
fn an_explicit_null_field_stays_null_and_is_not_absent() {
    let value = parse("{name: null}");
    assert_eq!(value, container(json!({ "name": null })));
    assert_eq!(parse("{ name: null, max-width: 1 }"), container(json!({ "name": null, "max-width": 1 })));
    assert_eq!(parse("{ name: , max-width: 1 }"), container(json!({ "name": null, "max-width": 1 })));
    assert_eq!(parse("{ max-width: 1, name: }"), container(json!({ "max-width": 1, "name": null })));
    assert_eq!(parse("{ name: ~, max-width: 1 }"), container(json!({ "name": null, "max-width": 1 })));
    // `name: string` rejects null, so the whole value is kept for validation.
    assert_eq!(value.bind(Some(&author())), json!({ "name": null }));
}

#[test]
fn a_wrong_typed_field_keeps_the_whole_value_for_validation() {
    both_loaders("age: 'abc'", json!({ "age": "abc" }));
    both_loaders("tags: 123", json!({ "tags": 123 }));
}

#[test]
fn one_invalid_element_is_kept_not_filtered() {
    both_loaders("tags: ['1', 'x']", json!({ "tags": ["1", "x"] }));
}

#[test]
fn every_invalid_element_is_kept_not_emptied() {
    both_loaders("tags: ['x', 'y']", json!({ "tags": ["x", "y"] }));
}

#[test]
fn an_empty_field_is_empty_not_absent() {
    both_loaders("name: '', tags: []", json!({ "name": "", "tags": [] }));
    assert_eq!(parse("{}"), container(json!({})));
    assert_eq!(parse("[]"), container(json!([])));
    assert_eq!(parse("{ max-width: {} }"), container(json!({ "max-width": {} })));
}

#[test]
fn an_unknown_key_is_kept_for_validation() {
    assert_eq!(parse("{mistake: 1}").bind(Some(&author())), json!({ "mistake": 1 }));
}

#[test]
fn a_number_in_a_container_has_already_lost_its_spelling() {
    // Quoting the nested value, or a dotted setter, is how to keep `1.10`.
    let shape = closed_object(vec![("version", Shape::text())]);
    assert_eq!(parse("{version:1.10}").bind(Some(&shape)), json!({ "version": "1.1" }));
    assert_eq!(parse("{version:'1.10'}").bind(Some(&shape)), json!({ "version": "1.10" }));
}
