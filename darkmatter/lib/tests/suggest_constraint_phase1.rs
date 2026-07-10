//! Executable acceptance tests for `suggest(...)`.
//!
//! All scaffolds are enabled and run under the package's ordinary L1 gate.

use std::str::FromStr;

use darkmatter::markdown::{
    Markdown,
    schemas::{
        DarkmatterSchemas, SchemaError, parse_yaml_schema,
        resolve::resolve_schema,
        simplified::{grammar::parse_type_expr, to_json_schema},
    },
};
use serde_json::{Number, Value, json};

fn schema_json(yaml: &str) -> Result<Value, SchemaError> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("valid YAML fixture");
    let schema = parse_yaml_schema(&value)?;
    to_json_schema(&schema)
}

fn grammar_error(property: &str, expression: &str) -> (String, std::ops::Range<usize>) {
    match parse_type_expr(property, expression).expect_err("expression must be rejected") {
        SchemaError::Grammar { message, span, .. } => (message, span),
        other => panic!("expected grammar error, got {other:?}"),
    }
}

#[test]
fn suggest_phase1_eligible_scalar_and_array_forms_parse() {
    for expression in [
        "string(suggest(red, green, 'blue gray'))",
        "number(suggest(0.25, 1))",
        "number(integer; suggest(80, 443))",
        "string(suggest(alpha, beta))[]",
        "number(suggest(1, 2, 3))[]",
    ] {
        parse_type_expr("value", expression)
            .unwrap_or_else(|error| panic!("eligible expression `{expression}`: {error}"));
    }

    for expression in [
        "numberlike(suggest(1))",
        "date(suggest(2026-07-09))",
        "boolean(suggest(true))",
        "enum(red; suggest(red))",
        "any(suggest(value))",
        "string[](suggest(alpha))",
    ] {
        let (message, _) = grammar_error("value", expression);
        assert!(
            message.contains("suggest") && message.contains("not valid"),
            "unsupported target should identify `suggest`: {message}"
        );
    }
}

#[test]
fn suggest_phase1_empty_list_is_a_structural_error() {
    let (message, span) = grammar_error("value", "string(suggest())");
    assert!(message.contains("at least one candidate"), "{message}");
    assert_eq!(&"string(suggest())"[span], "suggest()");
}

#[test]
fn suggest_phase1_cardinality_is_per_complete_property_definition() {
    let (message, second_span) = grammar_error(
        "value",
        "string(suggest(a); suggest(b))",
    );
    assert!(message.contains("at most one `suggest`"), "{message}");
    assert_eq!(&"string(suggest(a); suggest(b))"[second_span], "suggest(b)");

    let union: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        "value:\n  - string(suggest(a))\n  - string(suggest(b))\n",
    )
    .unwrap();
    let error = parse_yaml_schema(&union).expect_err("property union must reject two lists");
    assert!(error.to_string().contains("at most one `suggest`"), "{error}");

    let root_union: serde_yaml_ng::Value = serde_yaml_ng::from_str(
        "- value: string(suggest(a))\n- value: string(suggest(b))\n",
    )
    .unwrap();
    parse_yaml_schema(&root_union)
        .expect("separate root-union property definitions may each suggest values");
}

#[test]
fn suggest_phase1_duplicates_are_rejected_at_the_later_argument() {
    for (expression, later) in [
        ("string(suggest(12, \"12\"))", "\"12\""),
        ("number(suggest(3, \"3\"))", "\"3\""),
        ("number(suggest(3, 003))", "003"),
        ("number(suggest(3, 3.0))", "3.0"),
        ("number(suggest(0, -0.000))", "-0.000"),
    ] {
        let (message, span) = grammar_error("value", expression);
        assert!(message.contains("duplicate suggestion"), "{message}");
        assert_eq!(&expression[span], later, "later duplicate span for `{expression}`");
    }
}

#[test]
fn suggest_phase1_conversion_preserves_string_interpretation_and_order() {
    let schema = schema_json("value: string(suggest(Orange, 12, true, null, 'blue gray'))\n")
        .expect("suggestion schema converts");
    assert_eq!(
        schema["properties"]["value"]["anyOf"][1]["x-darkmatter-suggest"],
        json!(["Orange", "12", "true", "null", "blue gray"])
    );
    assert!(schema["properties"]["value"]["anyOf"][0].get("examples").is_none());

    let array = schema_json("value: number(suggest(1, 2.50))[]\n")
        .expect("array suggestion schema converts");
    assert_eq!(
        array["properties"]["value"]["anyOf"][1]["items"]["x-darkmatter-suggest"],
        json!([1, 2.5])
    );
}

#[test]
fn suggest_phase1_invalid_decimal_syntax_is_metadata() {
    let schema = schema_json(
        "value: number(suggest(1e3, +1, .5, 5., many, ' 1 ', 003.500))\n",
    )
    .expect("invalid candidate metadata does not prevent conversion");
    assert_eq!(
        schema["properties"]["value"]["anyOf"][1]["x-darkmatter-suggest"],
        json!(["1e3", "+1", ".5", "5.", "many", " 1 ", 3.5])
    );
}

fn exact_json_number(text: &str) -> bool {
    Number::from_str(text).is_ok_and(|number| number.to_string() == text)
}

fn observable_integer_boundary() -> (String, String) {
    let mut accepted = "9".to_string();
    for _ in 0..256 {
        let candidate = format!("{accepted}9");
        if !exact_json_number(&candidate) {
            return (accepted, candidate);
        }
        accepted = candidate;
    }
    panic!("JSON numeric model exposed no observable integer boundary");
}

fn observable_negative_integer_boundary() -> (String, String) {
    let mut accepted = "9".to_string();
    for _ in 0..256 {
        let candidate = format!("{accepted}9");
        if !exact_json_number(&format!("-{candidate}")) {
            return (accepted, candidate);
        }
        accepted = candidate;
    }
    panic!("JSON numeric model exposed no observable negative integer boundary");
}

fn observable_fraction_boundary() -> (String, String) {
    let mut accepted = "0.1".to_string();
    for digit in (1..=9).cycle().take(512) {
        let candidate = format!("{accepted}{digit}");
        if !exact_json_number(&candidate) {
            return (accepted, candidate);
        }
        accepted = candidate;
    }
    panic!("JSON numeric model exposed no observable fractional boundary");
}

#[test]
fn suggest_phase1_numeric_boundaries_follow_observable_json_round_trip() {
    let (integer, beyond_integer) = observable_integer_boundary();
    let (negative_integer, beyond_negative_integer) = observable_negative_integer_boundary();
    let (fraction, beyond_fraction) = observable_fraction_boundary();
    let yaml = format!(
        "value: number(suggest({integer}, {beyond_integer}, -{negative_integer}, -{beyond_negative_integer}, \
         {fraction}, {beyond_fraction}, 0003.5000, -0.000))\n"
    );
    let schema = schema_json(&yaml).expect("boundary metadata converts");
    let suggestions = &schema["properties"]["value"]["anyOf"][1]["x-darkmatter-suggest"];
    assert!(suggestions[0].is_number());
    assert_eq!(suggestions[1], json!(beyond_integer));
    assert!(suggestions[2].is_number());
    assert_eq!(suggestions[3], json!(format!("-{beyond_negative_integer}")));
    assert!(suggestions[4].is_number());
    assert_eq!(suggestions[5], json!(beyond_fraction));
    assert_eq!(suggestions[6], json!(3.5));
    assert_eq!(suggestions[7], json!(0));
}

#[test]
fn suggest_phase1_invalid_candidates_remain_loadable_and_lintable() {
    let schema = schema_json(
        "score: number(integer; min(0); max(100); suggest(-1, 50, 50.5, 101, many))\n\
         label: string(min(3); max(5); pattern(^[a-z]+$); suggest(a, valid, TOOLONG))\n",
    )
    .expect("invalid suggestions remain loadable");
    assert_eq!(
        schema["properties"]["score"]["anyOf"][1]["x-darkmatter-suggest"],
        json!([-1, 50, 50.5, 101, "many"])
    );
    assert_eq!(
        schema["properties"]["label"]["anyOf"][1]["x-darkmatter-suggest"],
        json!(["a", "valid", "TOOLONG"])
    );

    // Phase 3 replaces this annotation-level checkpoint with assertions over
    // the public span-bearing lint product. Keeping the schema conversion here
    // proves those defects are metadata, not SchemaError values.
}

#[test]
fn suggest_phase1_candidate_constraints_target_scalar_or_array_items() {
    let scalar = schema_json(
        "value: string(required; default(ok); example(this); min(2); suggest(x, ok))\n",
    )
    .expect("candidate constraints convert");
    assert_eq!(
        scalar["properties"]["value"]["x-darkmatter-suggest"],
        json!(["x", "ok"])
    );

    let array = schema_json("value: number(integer; min(0); suggest(-1, 1.5, 2))[]\n")
        .expect("array item constraints convert");
    let item = &array["properties"]["value"]["anyOf"][1]["items"];
    assert_eq!(item["x-darkmatter-suggest"], json!([-1, 1.5, 2]));
    assert_eq!(item["type"], json!("integer"));
    assert_eq!(item["minimum"], json!(0));
}

#[test]
fn suggest_phase1_metadata_does_not_restrict_document_values() {
    let document: Markdown = "---\n$schema:\n  color: string(suggest(red, green))\ncolor: purple\n---\n"
        .into();
    let report = DarkmatterSchemas::new()
        .validate(&document)
        .expect("suggestion metadata builds a validator");
    assert!(report.valid, "an unlisted valid value remains valid: {:?}", report.problems);
}

#[test]
fn suggest_phase1_standalone_envelopes_resolve_consistently() {
    let dir = tempfile::tempdir().unwrap();
    let pure = dir.path().join("pure.yaml");
    let tagged = dir.path().join("tagged.yaml");
    let sequence = dir.path().join("sequence.yaml");
    std::fs::write(&pure, "$schema:\n  name: string(suggest(Bob, Mary))\n").unwrap();
    std::fs::write(
        &tagged,
        "kind: schema\ntypes:\n  name: string(suggest(Bob, Mary))\n",
    )
    .unwrap();
    std::fs::write(
        &sequence,
        "$schema:\n  - kind: enum(person)\n  - kind: enum(team)\n",
    )
    .unwrap();

    let pure_resolved = resolve_schema(&json!(pure.to_string_lossy()), dir.path()).unwrap();
    let tagged_resolved = resolve_schema(&json!(tagged.to_string_lossy()), dir.path()).unwrap();
    assert_eq!(pure_resolved.json_schema, tagged_resolved.json_schema);
    assert!(pure_resolved.simplified.is_some());
    assert!(tagged_resolved.simplified.is_some());
    assert!(resolve_schema(&json!(sequence.to_string_lossy()), dir.path()).is_ok());
}

#[test]
fn suggest_phase1_fixture_newlines_are_explicit() {
    let inline = include_str!("../../dmls/tests/fixtures/suggest_constraint/inline.md");
    let completion = include_str!("../../dmls/tests/fixtures/suggest_constraint/completion.md");
    assert!(!inline.contains('\r') && inline.ends_with('\n'));
    assert!(!completion.contains('\r') && completion.ends_with('\n'));

    // CRLF coverage is derived without an OS-specific file checkout mode.
    let crlf = inline.replace('\n', "\r\n");
    assert!(crlf.contains("\r\n"));
    assert_eq!(crlf.replace("\r\n", "\n"), inline);
}
