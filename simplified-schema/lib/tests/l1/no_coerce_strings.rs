//! `string(no-coerce)` through the public API: soft validation accepts only
//! text as written and never turns a native number or boolean into text; a
//! union's other options keep their own conversions. Valid only on `string`.

use serde_json::{Value, json};
use simplified_schema::{SimplifiedSchema, ValidationFailure};

fn one_property(declaration: &str) -> SimplifiedSchema {
    let text = format!("$schema:\n  p: {declaration}\n");
    SimplifiedSchema::try_from(text.as_str()).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

/// The soft-validation problem paths of `{ p: value }`, or none when accepted.
fn soft_problems(schema: &SimplifiedSchema, value: Value) -> Vec<String> {
    match schema.validate_soft(&json!({ "p": value })) {
        Ok(_) => Vec::new(),
        Err(failure @ ValidationFailure::InvalidData { .. }) => {
            failure.problems().iter().map(|problem| problem.path.clone()).collect()
        }
        Err(other) => panic!("refused as data: {other:?}"),
    }
}

#[test]
fn a_native_scalar_is_refused_and_written_text_is_kept() {
    let strict_text = one_property("string(no-coerce)");
    let plain = one_property("string");

    for value in [json!(42), json!(true)] {
        assert_eq!(soft_problems(&strict_text, value.clone()), ["/p"], "{value}");
        assert!(soft_problems(&plain, value.clone()).is_empty(), "plain string still converts {value}");
    }
    let quoted = json!({ "p": "42" });
    let kept = strict_text.validate_soft(&quoted).expect("quoted text");
    assert!(!kept.is_modified());
}

#[test]
fn a_union_keeps_its_other_conversions() {
    let schema = one_property("\n    - string(no-coerce)\n    - number");
    let text = json!({ "p": "4" });
    let converted = schema.validate_soft(&text).expect("text is kept");
    assert_eq!(converted.value["p"], json!("4"));
    assert!(schema.validate_soft(&json!({ "p": 4 })).is_ok(), "the number arm accepts 4");
    assert_eq!(soft_problems(&one_property("\n    - string(no-coerce)\n    - string[]"), json!(true)), ["/p"]);
}

#[test]
fn it_lowers_to_an_annotation_and_round_trips() {
    let schema = one_property("string(no-coerce; required)");
    let lowered = schema.to_json_schema().expect("lowers");
    assert_eq!(lowered["properties"]["p"]["x-darkmatter-no-coerce"], json!(true), "{lowered}");
    let yaml = schema.to_simplified_yaml().expect("exports");
    assert!(yaml.contains("no-coerce"), "{yaml}");
    assert_eq!(SimplifiedSchema::try_from(yaml.as_str()).expect("re-parses").to_json_schema().expect("lowers"), lowered);
}

#[test]
fn it_is_refused_on_any_other_type() {
    for declaration in ["number(no-coerce)", "alphanumeric(no-coerce)", "boolean(no-coerce)"] {
        let text = format!("$schema:\n  p: {declaration}\n");
        let refused = SimplifiedSchema::try_from(text.as_str()).and_then(|schema| schema.to_json_schema().map(drop));
        assert!(refused.is_err(), "`{declaration}` is refused");
    }
}
