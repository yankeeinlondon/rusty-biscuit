//! Both translators feed one rule core: every grammar construct, bound
//! through the grammar translator (`call_binding`) and through its JSON
//! Schema lowering plus the schema adapter's translator, keeps, converts, or
//! rejects each probe value identically.

use darkmatter::markdown::coercion::{Bound, bind};
use darkmatter::markdown::compose::expression::call_binding::{NoNamedTypes, shape_from_simplified_schema};
use darkmatter::markdown::schemas::shape_from_json_schema;
use darkmatter::markdown::schemas::simplified::{parse_yaml_schema, to_json_schema};
use serde_json::{Value, json};

/// The keep/convert/error outcome, without diagnostic wording.
#[derive(Debug, PartialEq)]
enum Outcome {
    Kept,
    Converted(Value),
    Rejected,
}

fn outcome(result: Result<Bound<'_>, darkmatter::markdown::coercion::ConversionError>) -> Outcome {
    match result {
        Ok(Bound::Kept(_)) => Outcome::Kept,
        Ok(Bound::Converted(value)) => Outcome::Converted(value),
        Err(_) => Outcome::Rejected,
    }
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
        let through_grammar = outcome(bind(&grammar_shape, &document));
        let through_json = outcome(bind(&json_shape, &document));
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
}
