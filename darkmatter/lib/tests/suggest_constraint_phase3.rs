use darkmatter::markdown::{
    Markdown,
    schemas::{
        DarkmatterSchemas, SuggestionLintReason, lint_suggestions, parse_yaml_schema,
        simplified::{parse_yaml_schema_with_source, to_json_schema},
    },
};
use insta::assert_snapshot;
use serde_json::{Value, json};

fn parse(yaml: &str) -> darkmatter::markdown::schemas::SimplifiedSchema {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
    parse_yaml_schema(&value).unwrap()
}

fn convert(yaml: &str) -> Value {
    to_json_schema(&parse(yaml)).unwrap()
}

#[test]
fn generated_annotation_preserves_types_order_and_item_placement() {
    let strings = convert("value: string(required; suggest(Orange, 12, true, 'blue gray'))\n");
    assert_eq!(
        strings["properties"]["value"]["x-darkmatter-suggest"],
        json!(["Orange", "12", "true", "blue gray"])
    );
    let numbers = convert("value: number(suggest(1, 2.50, many))[]\n");
    let item = &numbers["properties"]["value"]["anyOf"][1]["items"];
    assert_eq!(item["x-darkmatter-suggest"], json!([1, 2.5, "many"]));

    for schema in [&strings, &numbers] {
        let text = serde_json::to_string(schema).unwrap();
        assert!(!text.contains("\"examples\""));
        assert!(!text.contains("x-darkmatter-example"));
        jsonschema::options()
            .build(schema)
            .expect("generated schema independently compiles");
    }
}

#[test]
fn conversion_snapshot_covers_valid_and_invalid_metadata() {
    let schema = convert(
        "label: string(required; min(2); suggest(ok, x))\n\
         score: number(required; integer; min(0); max(10); suggest(-1, 3, 3.5, 11, many))\n\
         tags: string(suggest(alpha, beta))[]\n",
    );
    assert_snapshot!(serde_json::to_string_pretty(&schema).unwrap());
}

#[test]
fn lint_is_public_span_bearing_and_deterministic_across_root_arms() {
    let yaml = concat!(
        "- score: number(integer; min(0); max(10); suggest(-1, 3.5, 11, many))\n",
        "- score: number(suggest(1e3))\n",
        "  label: string(min(3); suggest(x))\n",
    );
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).unwrap();
    let schema = parse_yaml_schema_with_source(&value, yaml, 23).unwrap();
    let problems = lint_suggestions(&schema).expect("invalid metadata is lint data");

    assert_eq!(
        problems
            .iter()
            .map(|problem| (problem.root_arm, problem.property.as_str(), problem.reason))
            .collect::<Vec<_>>(),
        vec![
            (Some(0), "score", SuggestionLintReason::Range),
            (Some(0), "score", SuggestionLintReason::Integer),
            (Some(0), "score", SuggestionLintReason::Range),
            (Some(0), "score", SuggestionLintReason::InvalidDecimalSyntax),
            (Some(1), "score", SuggestionLintReason::InvalidDecimalSyntax),
            (Some(1), "label", SuggestionLintReason::Length),
        ]
    );
    for problem in &problems {
        assert_eq!(&yaml[problem.span.start - 23..problem.span.end - 23], match problem.decoded.as_str() {
            "-1" => "-1",
            "3.5" => "3.5",
            "11" => "11",
            "many" => "many",
            "1e3" => "1e3",
            "x" => "x",
            other => panic!("unexpected candidate {other}"),
        });
    }
}

#[test]
fn lint_categorizes_number_and_string_constraints() {
    let mut beyond = "9".to_string();
    loop {
        let candidate = format!("{beyond}9");
        let schema = parse(&format!("value: number(suggest({candidate}))\n"));
        if lint_suggestions(&schema).unwrap().first().is_some_and(|problem| {
            problem.reason == SuggestionLintReason::UnsupportedNumberRepresentation
        }) {
            beyond = candidate;
            break;
        }
        beyond = candidate;
        assert!(beyond.len() < 256);
    }

    let cases = [
        ("value: string(not-empty; suggest('   '))\n".to_string(), SuggestionLintReason::NotEmpty),
        ("value: string(pattern(^[a-z]+$); suggest(BAD))\n".to_string(), SuggestionLintReason::Pattern),
        (format!("value: number(suggest({beyond}))\n"), SuggestionLintReason::UnsupportedNumberRepresentation),
    ];
    for (yaml, expected) in cases {
        let problems = lint_suggestions(&parse(&yaml)).unwrap();
        assert_eq!(problems.len(), 1, "{yaml}");
        assert_eq!(problems[0].reason, expected, "{yaml}");
    }
}

#[test]
fn suggestion_metadata_neither_restricts_validation_nor_blocks_composition() {
    let document: Markdown = "---\n$schema:\n  score: number(integer; min(0); suggest(-1, 2.5, many))\n  color: string(suggest(red, green))\nscore: 42\ncolor: purple\n---\n\n# Valid\n"
        .into();
    let schemas = DarkmatterSchemas::new();
    let effective = schemas
        .effective_for(&document)
        .expect("invalid suggestions do not block resolution")
        .expect("inline schema is effective");
    assert!(effective.simplified.is_some());
    let report = schemas.validate(&document).expect("validator construction succeeds");
    assert!(report.valid, "unlisted document values remain valid: {:?}", report.problems);

    let (composed, _) = document.compose().expect("composition remains permissive");
    assert!(composed.as_string().contains("color: purple"));
}
