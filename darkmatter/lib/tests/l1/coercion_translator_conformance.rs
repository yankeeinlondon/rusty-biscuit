//! Both translators feed one rule core: every grammar construct, bound
//! through the grammar translator (`call_binding`) and through its JSON
//! Schema lowering plus the schema adapter's translator, keeps, converts, or
//! rejects each probe value identically, both plainly and with a template
//! test consulted at the places marked `deferred`. Recursive named types are
//! bound through the resolved schema: its definition table on the grammar
//! side, its lowered `$defs` on the JSON Schema side.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::coercion::{BindOptions, Bound, ConversionError, PathSegment, Shape, bind, bind_with_marked};
use darkmatter::markdown::compose::expression::call_binding::{
    NoNamedTypes, shape_from_simplified_schema, shape_from_simplified_schema_with_definitions,
};
use darkmatter::markdown::compose::expression::is_deferred_template;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::shape_from_json_schema;
use darkmatter::markdown::schemas::simplified::{parse_yaml_schema, to_json_schema};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The keep/convert/error outcome, without diagnostic wording.
#[derive(Debug, PartialEq)]
enum Outcome {
    Kept,
    Converted(Value),
    Rejected,
}

fn outcome(result: Result<Bound<'_>, ConversionError>) -> Outcome {
    match result {
        Ok(Bound::Kept(_)) => Outcome::Kept,
        Ok(Bound::Converted(value)) => Outcome::Converted(value),
        Err(_) => Outcome::Rejected,
    }
}

/// The template test schema validation consults at `deferred` places.
fn template(_: &[PathSegment], value: &Value) -> bool {
    value.as_str().is_some_and(is_deferred_template)
}

/// The outcome of binding `value` plainly and with [`template`] consulted at
/// the shape's `deferred` places.
fn outcomes(shape: &Shape, value: &Value) -> (Outcome, Outcome) {
    let marked = bind_with_marked(shape, value, &BindOptions::default(), Some(&template));
    (outcome(bind(shape, value)), outcome(marked))
}

fn probes() -> Vec<Value> {
    vec![
        json!("4"),
        json!(4),
        json!(" 4 "),
        json!("1"),
        json!("yes"),
        json!("On"),
        json!(true),
        json!(null),
        json!(2026),
        json!(1.5),
        json!("2.5"),
        json!("pear"),
        json!("abc"),
        json!("ABCD"),
        json!(123),
        json!("9007199254740993"),
        json!("1e20"),
        json!(1e20),
        json!("02134"),
        json!("2026-10-22"),
        json!("https://example.com"),
        json!("ada@example.com"),
        json!("{\"a\":1}"),
        json!("{a"),
        json!("port: 8080"),
        json!({"port": 8080, "debug": true}),
        json!({"id": "1"}),
        json!({"a": "1", "b": "1"}),
        json!({"inner": {"count": "3"}}),
        json!({"x_a": "yes", "n": "2"}),
        json!([]),
        json!(["1", "2"]),
        json!(["2", "yes"]),
        json!([true]),
        json!(["Ada", "36", "yes"]),
        json!(["Ada", "36"]),
        json!(["Ada"]),
        json!(["Ada", "36", "yes", "extra"]),
        json!("{{ fm.count }}"),
        json!("n={{ fm.count }}"),
        json!("{{ 1 + }}"),
        json!(["{{ x }}", "2"]),
        json!({"id": "{{ x }}"}),
    ]
}

/// Binds every probe, as the single property `v`, through both translators.
fn assert_translators_agree(definition: &str) {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&format!("v: {definition}")).expect("the definition is YAML");
    let schema = parse_yaml_schema(&yaml).expect("the definition parses");
    let grammar_shape = shape_from_simplified_schema(&schema, &NoNamedTypes).expect("grammar translates");
    let json_schema = to_json_schema(&schema).expect("the definition lowers");
    let json_shape = shape_from_json_schema(&json_schema);
    let mut disagreements = Vec::new();
    for probe in probes() {
        let document = json!({ "v": probe });
        let through_grammar = outcomes(&grammar_shape, &document);
        let through_json = outcomes(&json_shape, &document);
        if through_grammar != through_json {
            disagreements.push(format!("{probe}: grammar {through_grammar:?}, JSON Schema {through_json:?}"));
        }
    }
    assert!(disagreements.is_empty(), "`{definition}` disagrees:\n{}", disagreements.join("\n"));
}

macro_rules! conformance {
    ($($name:ident => $definition:expr;)+) => {
        $(
            #[test]
            fn $name() {
                assert_translators_agree($definition);
            }
        )+
    };
}

conformance! {
    string => "string";
    string_bounded => "\"string(min(3); max(3))\"";
    string_pattern => "\"string(pattern(^[a-z]+$))\"";
    number => "number";
    number_integer => "\"number(integer)\"";
    number_integer_min => "\"number(integer; min(1))\"";
    number_bounded => "\"number(min(5); max(10))\"";
    numberlike => "numberlike";
    boolean => "boolean";
    boolish => "boolish";
    enumeration => "\"enum(1, 2)\"";
    literal_number => "\"literal(2)\"";
    literal_boolean => "\"literal(true)\"";
    json_content => "json";
    yaml_content => "yaml";
    date => "date";
    url => "url";
    email => "email";
    file_reference => "file";
    alphanumeric => "alphanumeric";
    unknown => "unknown";
    opaque_object => "object";
    string_list => "\"string[]\"";
    number_list => "\"number[]\"";
    grouped_union_list => "\"(boolean | number)[]\"";
    tuple => "\"tuple([string, number, boolean?])\"";
    tuple_spread => "\"tuple([number, ...string[]])\"";
    inline_object => "\"{ id: number }\"";
    nested_object => "\"{ inner: { count: number } }\"";
    dictionary => "\"{ <string>: number }\"";
    prefixed_dictionary => "\"{ <starting::x_>: boolean, n: number }\"";
    union_operator => "\"string | number\"";
    union_text_or_list => "\"string | string[]\"";
    union_number_boolean => "\"number | boolean\"";
    union_constrained => "\"number(min(5)) | boolean\"";
    union_content => "\"json | yaml\"";
    union_objects => "\"{ id: number } | { id: boolean }\"";
    union_crossed_objects => "\"{ a: number, b: boolean } | { a: boolean, b: number }\"";
    union_yaml_list => "[\"number\", \"boolean\"]";
    nullable_number => "\"number | null\"";
    deferred_number => "\"number(deferred)\"";
    deferred_required_number => "\"number(required; deferred)\"";
    deferred_items => "\"number(deferred)[]\"";
    deferred_list => "\"number[](deferred)\"";
    deferred_union_arms => "\"number(integer; deferred) | object(deferred)\"";
    deferred_one_union_arm => "\"number(deferred) | boolean\"";
    deferred_tuple_slot => "\"tuple([number(deferred), boolean])\"";
    deferred_property => "\"{ id: number(deferred) }\"";
    deferred_file => "\"file(deferred)\"";
    deferred_tuple => "\"tuple([number, number]; deferred)\"";
}

/// The `deferred` cases above agree because both translators mark the same
/// place, not because neither does: a marked number keeps a template only
/// when the template test is consulted, and an unmarked one never does.
#[test]
fn deferred_marks_only_its_own_place_in_both_translators() {
    for (definition, value, expected) in [
        ("\"number(deferred)\"", json!({ "v": "{{ x }}" }), (Outcome::Rejected, Outcome::Kept)),
        ("number", json!({ "v": "{{ x }}" }), (Outcome::Rejected, Outcome::Rejected)),
        ("\"number(deferred)[]\"", json!({ "v": ["{{ x }}", "2"] }), (Outcome::Rejected, Outcome::Converted(json!({ "v": ["{{ x }}", 2] })))),
        ("\"number[](deferred)\"", json!({ "v": ["{{ x }}"] }), (Outcome::Rejected, Outcome::Rejected)),
        ("\"number[](deferred)\"", json!({ "v": "{{ x }}" }), (Outcome::Rejected, Outcome::Kept)),
        ("\"number(deferred) | boolean\"", json!({ "v": "{{ x }}" }), (Outcome::Rejected, Outcome::Kept)),
        ("\"number | boolean\"", json!({ "v": "{{ x }}" }), (Outcome::Rejected, Outcome::Rejected)),
    ] {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&format!("v: {definition}")).expect("YAML");
        let schema = parse_yaml_schema(&yaml).expect("the definition parses");
        let grammar_shape = shape_from_simplified_schema(&schema, &NoNamedTypes).expect("grammar translates");
        let json_shape = shape_from_json_schema(&to_json_schema(&schema).expect("the definition lowers"));
        assert_eq!(outcomes(&grammar_shape, &value), expected, "grammar: {definition} {value}");
        assert_eq!(outcomes(&json_shape, &value), expected, "JSON Schema: {definition} {value}");
    }
}

/// Binds every probe through both translators of the resolved inline
/// `schema`, whose named types recurse.
fn assert_recursive_translators_agree(schema: &str, probes: &[Value]) {
    let dir = TempDir::new().expect("tempdir");
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(schema).expect("the schema is YAML");
    let resolved =
        resolve_yaml_schema(&yaml, dir.path(), &FileResolutionContext::new(dir.path())).expect("the schema resolves");
    assert!(!resolved.definitions.is_empty(), "the fixture recurses");
    let grammar_shape = shape_from_simplified_schema_with_definitions(
        resolved.simplified.as_ref().expect("a simplified schema"),
        &NoNamedTypes,
        &resolved.definitions,
    )
    .expect("grammar translates");
    let json_shape = shape_from_json_schema(&resolved.json_schema);
    let mut disagreements = Vec::new();
    for probe in probes {
        let through_grammar = outcomes(&grammar_shape, probe);
        let through_json = outcomes(&json_shape, probe);
        if through_grammar != through_json {
            disagreements.push(format!("{probe}: grammar {through_grammar:?}, JSON Schema {through_json:?}"));
        }
    }
    assert!(disagreements.is_empty(), "`{schema}` disagrees:\n{}", disagreements.join("\n"));
}

/// A `{ value, children }` chain `depth` levels deep with `leaf` at the
/// bottom; nine levels is below the JSON translator's old expansion cutoff.
fn tree(depth: usize, leaf: Value) -> Value {
    (1..depth).fold(json!({"value": leaf, "children": []}), |child, level| {
        json!({"value": format!("n{level}"), "children": [child]})
    })
}

#[test]
fn recursive_object_through_a_list() {
    let schema = "item: \"{ value: string(required), children: item[](required) }\"\ntree: item(required)\n";
    let mut probes = Vec::new();
    for depth in [1, 6, 9] {
        for leaf in [json!("x"), json!(42), json!(true), json!(null), json!({"x": 1}), json!(["x"])] {
            probes.push(json!({ "tree": tree(depth, leaf) }));
        }
    }
    probes.push(json!({ "tree": tree(40, json!("x")) }));
    probes.push(json!({ "tree": tree(40, json!(42)) }));
    probes.push(json!({ "tree": { "value": 1, "children": [tree(9, json!(2)), { "value": 3 }] } }));
    probes.push(json!({ "tree": { "value": "a", "children": [tree(9, json!(2)), { "value": "b", "children": [], "extra": 1 }] } }));
    probes.push(json!({ "tree": { "value": "a", "children": [tree(9, json!(2))] }, "item": tree(7, json!(false)) }));
    probes.push(json!({}));
    assert_recursive_translators_agree(schema, &probes);
}

#[test]
fn recursive_list_through_a_union_item() {
    let schema = "nested: \"(number | nested)[]\"\nv: nested(required)\n";
    let deep = |leaf: Value| (0..9).fold(json!([leaf]), |inner, _| json!(["1", inner]));
    let probes = [
        json!({ "v": [] }),
        json!({ "v": ["1", 2, ["3"]] }),
        json!({ "v": deep(json!("4")) }),
        json!({ "v": deep(json!(true)) }),
        json!({ "v": deep(json!("pear")) }),
        json!({ "v": deep(json!({"a": 1})) }),
        json!({ "v": "1" }),
    ];
    assert_recursive_translators_agree(schema, &probes);
}

#[test]
fn mutually_recursive_objects_through_optional_properties() {
    let schema = "a: \"{ n: number, b: b }\"\nb: \"{ flag: boolean, a: a }\"\nroot: a(required)\n";
    let chain = |leaf: Value| {
        (0..5).fold(json!({ "n": "1", "b": { "flag": leaf } }), |inner, _| {
            json!({ "n": "2", "b": { "flag": "yes", "a": inner } })
        })
    };
    let probes = [
        json!({ "root": {} }),
        json!({ "root": chain(json!("true")) }),
        json!({ "root": chain(json!("pear")) }),
        json!({ "root": chain(json!(null)) }),
        json!({ "root": { "n": null, "b": null } }),
        json!({ "root": { "n": 1, "b": { "flag": true, "a": { "n": "x" } } } }),
    ];
    assert_recursive_translators_agree(schema, &probes);
}

#[test]
fn ambiguous_union_inside_a_recursive_type() {
    let schema = "pair: \"{ a: number, b: boolean } | { a: boolean, b: number }\"\n\
                  node: \"{ value: pair(required), children: node[](required) }\"\ntree: node(required)\n";
    let chain = |leaf: Value| {
        (1..9).fold(json!({ "value": leaf, "children": [] }), |child, _| {
            json!({ "value": { "a": 1, "b": true }, "children": [child] })
        })
    };
    let probes = [
        json!({ "tree": chain(json!({ "a": "1", "b": "1" })) }),
        json!({ "tree": chain(json!({ "a": "1", "b": true })) }),
        json!({ "tree": chain(json!({ "a": true, "b": "2" })) }),
        json!({ "tree": chain(json!({ "a": "x", "b": "y" })) }),
    ];
    assert_recursive_translators_agree(schema, &probes);
}

#[test]
fn deferred_place_inside_a_recursive_type() {
    let schema = "node: \"{ n: number(deferred), m: number, kids: node[] }\"\ntree: node(required)\n";
    let chain = |leaf: Value| {
        (0..6).fold(json!({ "n": leaf.clone(), "m": leaf }), |inner, _| json!({ "n": "1", "kids": [inner] }))
    };
    let probes = [
        json!({ "tree": chain(json!("{{ x }}")) }),
        json!({ "tree": chain(json!("2")) }),
        json!({ "tree": chain(json!("pear")) }),
        json!({ "tree": { "n": "{{ x }}" } }),
        json!({ "tree": { "m": "{{ x }}" } }),
    ];
    assert_recursive_translators_agree(schema, &probes);
}

#[test]
fn use_site_constraints_beside_a_recursive_reference() {
    let schema = "item: \"{ value: string(required), children: item[] }\"\n\
                  one: \"item(required; max-keys(1))\"\n\
                  kids: \"item[](max(1))\"\n";
    let probes = [
        json!({ "one": { "value": 1 } }),
        json!({ "one": { "value": "1", "children": [] } }),
        json!({ "one": { "value": 1 }, "kids": [tree(9, json!(2))] }),
        json!({ "one": { "value": 1 }, "kids": [{ "value": 2 }, { "value": 3 }] }),
    ];
    assert_recursive_translators_agree(schema, &probes);
}
