//! Prepare-time nested-span validation (`validate_no_nested_spans_in_literals`).

use super::*;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{ParseMode, lint_expression};

const REVIEW_SPEC_INLINE: &str = include_str!(
    "../../../../../cli/tests/fixtures/nested_span_regression/review-spec-inline.md"
);
const COMMIT: &str =
    include_str!("../../../../../cli/tests/fixtures/nested_span_regression/commit.md");

fn fixture_frontmatter(text: &str) -> serde_json::Value {
    let md = Markdown::from(text);
    serde_json::Value::Object(
        md.frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

fn validate(frontmatter: &serde_json::Value) -> Result<(), CompositionError> {
    let config = parse_lifecycle_config(frontmatter, dummy_path()).expect("lifecycle parses");
    validate_no_nested_spans_in_literals(frontmatter, &config, dummy_path())
}

/// The rejected property, literal, nested span, and suggestion.
fn rejection(frontmatter: &serde_json::Value) -> (String, String, String, Option<String>) {
    match validate(frontmatter) {
        Err(CompositionError::LifecycleNestedSpanInLiteral {
            property,
            literal,
            nested,
            suggestion,
            ..
        }) => (property, literal, nested, suggestion),
        other => panic!("expected LifecycleNestedSpanInLiteral, got {other:?}"),
    }
}

fn inner_expression(whole_value: &str) -> String {
    ExpressionFinder::find_all_plain(whole_value)
        .into_iter()
        .next()
        .expect("whole-value span")
        .expression
}

#[test]
fn incident_is_rejected_at_success_say_with_the_shared_rewrite() {
    let frontmatter = fixture_frontmatter(REVIEW_SPEC_INLINE);
    let (property, literal, nested, suggestion) = rejection(&frontmatter);

    assert_eq!(property, "success.say");
    assert_eq!(
        literal,
        r#""The review of the draft specification file in {{ctx.area}} has completed""#
    );
    assert_eq!(nested, "{{ctx.area}}");

    let say = frontmatter["success"]["say"].as_str().unwrap();
    let expected = lint_expression(&inner_expression(say), ParseMode::Interpolation)[0]
        .suggestion
        .clone();
    assert_eq!(suggestion, expected, "Claudine must carry Darkmatter's rewrite bytes");
    let suggestion = suggestion.expect("incident has a rewrite");
    assert!(suggestion.contains(r#""The review of the draft specification file in " + ctx.area + " has completed""#));
    assert!(suggestion.contains(r#"" repo has completed""#));
    assert!(!suggestion.contains("{{"), "{suggestion}");
}

#[test]
fn incident_failure_say_is_reported_once_success_say_is_repaired() {
    let mut frontmatter = fixture_frontmatter(REVIEW_SPEC_INLINE);
    frontmatter["success"]["say"] =
        json!("{{ ctx.area ? 'done in ' + ctx.area : 'done in ' + ctx.repo_name }}");
    let (property, _, nested, suggestion) = rejection(&frontmatter);
    assert_eq!(property, "failure.say");
    assert_eq!(nested, "{{ title_case(without_date(parent_dir(spec))) }}");
    assert!(suggestion.unwrap().contains("title_case(without_date(parent_dir(spec)))"));

    frontmatter["failure"]["say"] = json!("{{ 'failed in ' + ctx.repo_name }}");
    validate(&frontmatter).expect("a fully repaired incident validates");
}

#[test]
fn every_communication_field_on_every_event_is_checked() {
    let defect = "{{ ok ? 'in {{x}}' : 'y' }}";
    for event in ["initialize", "start", "success", "blocked", "failure", "finalize"] {
        for field in ["say", "say_first", "message", "stderr", "notify", "info", "warn", "success", "stdout"] {
            let frontmatter = json!({ event: { field: defect } });
            let (property, literal, nested, _) = rejection(&frontmatter);
            assert_eq!(property, format!("{event}.{field}"));
            assert_eq!(literal, "'in {{x}}'");
            assert_eq!(nested, "{{x}}");
        }
    }
    let frontmatter = json!({ "loop": { "while": "true", "info": defect } });
    assert_eq!(rejection(&frontmatter).0, "loop.info");
}

#[test]
fn predicates_are_linted_as_condition_text() {
    let frontmatter = json!({
        "start": { "stack": [{ "when": "label == 'in {{x}}'", "action": "stop" }] }
    });
    let (property, literal, nested, suggestion) = rejection(&frontmatter);
    assert_eq!(property, "start.stack[0].when");
    assert_eq!(literal, "'in {{x}}'");
    assert_eq!(nested, "{{x}}");
    assert_eq!(suggestion.as_deref(), Some("label == 'in ' + x"));

    for predicate in ["while", "until"] {
        let frontmatter = json!({ "loop": { predicate: "status != 'at {{step}}'" } });
        assert_eq!(rejection(&frontmatter).0, format!("loop.{predicate}"));
    }
}

#[test]
fn action_operands_are_checked_in_every_authoring_form() {
    let defect = "{{ ok ? 'a {{x}}' : 'b' }}";
    let cases = [
        (json!({ "say": defect }), "start.stack[0].action[0].message"),
        (json!({ "action": "say", "message": defect }), "start.stack[0].action[0].message"),
        (json!({ "action": "notify", "text": defect }), "start.stack[0].action[0].message"),
        (json!({ "shell": defect }), "start.stack[0].action[0].command"),
        (json!({ "action": "shell", "command": "true", "on_error": defect }), "start.stack[0].action[0].on_error"),
        (json!({ "error": defect }), "start.stack[0].action[0].reason"),
        (json!({ "proxy": defect }), "start.stack[0].action[0].target"),
        (json!({ "action": "resume", "message": defect }), "start.stack[0].action[0].message"),
        (json!({ "set_frontmatter": ["s.md", "k", defect] }), "start.stack[0].action[0].arg[2]"),
        (json!({ "action": "set_frontmatter", "file": "s.md", "prop": "k", "value": defect }), "start.stack[0].action[0].arg[2]"),
    ];
    for (action, expected) in cases {
        let frontmatter = json!({ "start": { "stack": [{ "action": [action.clone()] }] } });
        assert_eq!(rejection(&frontmatter).0, expected, "action {action}");
    }
}

#[test]
fn proxy_with_values_are_checked_at_their_overlay_path() {
    let frontmatter = json!({
        "failure": { "stack": [{ "action": [
            { "say": "handing off" },
            { "action": "proxy", "target": "next.md", "with": {
                "note": "plain",
                "meta": { "files": ["a.md", "{{ 'in {{area}}' }}"] }
            } }
        ] }] }
    });
    let (property, _, nested, suggestion) = rejection(&frontmatter);
    assert_eq!(property, "failure.stack[0].action[1].with.meta.files[1]");
    assert_eq!(nested, "{{area}}");
    assert_eq!(suggestion.as_deref(), Some("'in ' + area"));
}

#[test]
fn events_are_reported_in_lifecycle_signal_order() {
    let defect = "{{ 'x {{y}}' }}";
    let frontmatter = json!({
        "finalize": { "say": defect },
        "success": { "stack": [{ "action": [{ "say": defect }] }] },
        "start": { "info": "fine", "stack": [{ "when": "a == 'b {{c}}'", "action": "stop" }] },
    });
    assert_eq!(rejection(&frontmatter).0, "start.stack[0].when");
}

#[test]
fn synthesized_mixed_and_non_lifecycle_surfaces_are_not_rejected() {
    let frontmatter = json!({
        "resides_in": "{{ ok ? 'in {{x}}' : 'y' }}",
        "start": {
            "info": "running {{agent}}",
            "message": "a {{ x ? 'in {{x}}' : 'y' }} b",
            "say": "{{{ 'literal {{braces}}' }}}",
            "stack": [{ "action": [
                { "info": "running {{agent}}" },
                { "action": "message", "message": "Deployed {{version}}" },
                { "set_frontmatter": ["s.md", "k", "{{ payload }}"] },
                { "say": "{{ 'in ' + area }}" },
                { "retry": 3 }
            ] }]
        }
    });
    validate(&frontmatter).expect("only single-pass authored literals are rejected");
}

#[test]
fn commit_fixture_nested_span_lives_on_a_non_lifecycle_key() {
    // `resides_in` is ordinary frontmatter, which its owning pipeline may
    // rescan, so it is outside Claudine's single-pass inventory.
    validate(&fixture_frontmatter(COMMIT)).expect("resides_in is not a lifecycle surface");
}

#[test]
fn a_surface_without_an_authored_source_is_an_internal_error() {
    let authored = json!({ "start": { "stack": [{ "action": [{ "say": "{{ 'x' }}" }] }] } });
    let config = parse_lifecycle_config(&authored, dummy_path()).unwrap();
    let err = validate_no_nested_spans_in_literals(&json!({}), &config, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleInvalid { property, message, .. } => {
            assert_eq!(property, "start.stack[0].action[0].message");
            assert!(message.contains("internal error"), "{message}");
        }
        other => panic!("expected internal LifecycleInvalid, got {other:?}"),
    }
}

#[test]
fn nested_span_error_renders_property_literal_rewrite_and_escape_hint() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let err = validate(&fixture_frontmatter(REVIEW_SPEC_INLINE)).unwrap_err();
    let rendered = strip_escape_codes(err.report_block_error_optimistic(Some(200)));
    assert!(rendered.contains("nested interpolation inside a string literal"), "{rendered}");
    assert!(rendered.contains("success.say"), "{rendered}");
    assert!(rendered.contains("{{ctx.area}}"), "{rendered}");
    assert!(rendered.contains(r#"" + ctx.area + ""#), "{rendered}");
    assert!(rendered.contains("{{{ … }}}"), "{rendered}");
    assert!(!rendered.contains(r#"\""#), "quotes must render unescaped: {rendered}");
    assert!(!rendered.contains("resolve the missing"), "{rendered}");
}

/// The single-pass inventory is checked against the authored Claudine schema,
/// so a new lifecycle event or communication field cannot silently escape the
/// nested-span rule (spec Invariant 2).
#[test]
fn single_pass_inventory_matches_the_authored_claudine_schema() {
    use std::collections::BTreeSet;

    let schema_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../darkmatter/docs/schemas");
    let load = |name: &str| -> serde_json::Value {
        let text = std::fs::read_to_string(schema_dir.join(name)).expect("schema readable");
        biscuit_file::serde_yaml_ng::from_str(&text).expect("schema is YAML")
    };
    let claudine = load("claudine.yaml");
    let types = load("claudine-types.yaml");

    let schema_events: BTreeSet<&str> = claudine["$schema"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, value)| {
            value.as_str().is_some_and(|ty| {
                ty.starts_with("lifecycle-event@") || ty.starts_with("loop-event@")
            })
        })
        .map(|(key, _)| key.as_str())
        .collect();
    let signals: BTreeSet<&str> = LifecycleSignal::ALL.iter().map(|s| s.property_name()).collect();
    assert_eq!(schema_events, signals);

    let comm_fields: BTreeSet<&str> = LIFECYCLE_COMM_FIELDS.iter().copied().collect();
    for event_type in ["lifecycle-event", "loop-event"] {
        let fields: BTreeSet<&str> = types["$schema"][event_type]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, value)| value.as_str().is_some_and(|ty| ty.starts_with("string ")))
            .map(|(key, _)| key.as_str())
            .filter(|key| *key != "effect")
            .collect();
        assert_eq!(fields, comm_fields, "{event_type} communication fields");
    }
    for predicate in ["while", "until"] {
        let ty = types["$schema"]["loop-event"][predicate].as_str().unwrap();
        assert!(ty.starts_with("expression "), "loop.{predicate}: {ty}");
    }
    let when = types["$schema"]["lifecycle-stack-item"]["when"].as_str().unwrap();
    assert!(when.starts_with("expression "), "stack when: {when}");
}
