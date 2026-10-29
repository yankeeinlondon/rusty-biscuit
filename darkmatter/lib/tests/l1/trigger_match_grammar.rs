//! The schema-trigger match grammar after the AND/OR change
//! (`2026-10-27-darkmatter-lifecycle-ownership`, criterion 18): a top-level
//! `match:` list is AND, a `group:` is OR, `group` nests inside `none`, `all`,
//! and `min-match` and they nest inside it, an anonymous nested list is
//! rejected, the vacuity rule judges the whole expression, and `any:` fails
//! naming `group`.
//!
//! Each case loads a whole envelope through `parse_trigger_envelope_from_str`
//! and judges frontmatter with [`MatchConditions::matches`], the evaluation
//! discovery and DMLS share.

use darkmatter::markdown::schemas::triggers::{MatchConditions, PathSubject, parse_trigger_envelope_from_str};
use darkmatter::markdown::schemas::SchemaError;
use serde_json::{Value, json};

fn load(match_body: &str) -> Result<MatchConditions, SchemaError> {
    parse_trigger_envelope_from_str(&format!("kind: schema-trigger\nmatch:\n{match_body}"))
        .map(|envelope| envelope.expect("claimed").conditions)
}

fn conditions(match_body: &str) -> MatchConditions {
    load(match_body).unwrap_or_else(|error| panic!("{match_body}\n{error}"))
}

fn selects(conditions: &MatchConditions, frontmatter: Value) -> bool {
    conditions.matches(&frontmatter, &PathSubject::detached())
}

fn message(match_body: &str) -> String {
    match load(match_body) {
        Err(error) => error.to_string(),
        Ok(conditions) => panic!("{match_body} loaded: {conditions:?}"),
    }
}

#[test]
fn a_top_level_list_is_and_while_a_group_is_or() {
    let list = conditions("  - a: string(required)\n  - b: string(required)\n");
    let group = conditions("  group:\n    - a: string(required)\n    - b: string(required)\n");
    for (frontmatter, and, or) in [
        (json!({ "a": "x", "b": "y" }), true, true),
        (json!({ "a": "x" }), false, true),
        (json!({ "b": "y" }), false, true),
        (json!({}), false, false),
    ] {
        assert_eq!(selects(&list, frontmatter.clone()), and, "list {frontmatter}");
        assert_eq!(selects(&group, frontmatter.clone()), or, "group {frontmatter}");
    }
}

#[test]
fn a_mapping_match_stays_one_and_expression() {
    let mapping = conditions("  a: string(required)\n  b: string(required)\n");
    assert!(selects(&mapping, json!({ "a": "x", "b": "y" })));
    assert!(!selects(&mapping, json!({ "a": "x" })));
}

#[test]
fn the_specification_example_selects_kind_and_either_prompt_or_steps() {
    let example = conditions(
        "  - kind: enum(claudine; required)\n  - group:\n      - prompt: string(required)\n      - steps: unknown[](required)\n",
    );
    assert!(selects(&example, json!({ "kind": "claudine", "prompt": "p" })));
    assert!(selects(&example, json!({ "kind": "claudine", "steps": [] })));
    assert!(!selects(&example, json!({ "kind": "claudine" })));
    assert!(!selects(&example, json!({ "prompt": "p" })));
}

#[test]
fn group_nests_inside_none_all_and_min_match() {
    let in_none = conditions(
        "  - gate: string(required)\n  - none:\n      - group:\n          - a: string(required)\n          - b: string(required)\n",
    );
    assert!(selects(&in_none, json!({ "gate": "g" })));
    assert!(!selects(&in_none, json!({ "gate": "g", "a": "x" })));
    assert!(!selects(&in_none, json!({ "gate": "g", "b": "y" })));

    let in_all = conditions("  all:\n    - gate: string(required)\n    - group:\n        - a: string(required)\n        - b: string(required)\n");
    assert!(selects(&in_all, json!({ "gate": "g", "b": "y" })));
    assert!(!selects(&in_all, json!({ "gate": "g" })));

    let in_min_match = conditions(
        "  min-match:\n    count: 2\n    of:\n      - c: string(required)\n      - group:\n          - a: string(required)\n          - b: string(required)\n      - d: string(required)\n",
    );
    assert!(selects(&in_min_match, json!({ "c": "x", "b": "y" })));
    assert!(selects(&in_min_match, json!({ "c": "x", "d": "z" })));
    assert!(!selects(&in_min_match, json!({ "a": "x", "b": "y" })), "one group counts once");
}

#[test]
fn none_all_min_match_and_path_nest_inside_group() {
    let group = conditions(
        "  group:\n    - all:\n        - a: string(required)\n        - none:\n            - b: string(required)\n    - min-match:\n        count: 2\n        of:\n          - c: string(required)\n          - d: string(required)\n          - e: string(required)\n",
    );
    assert!(selects(&group, json!({ "a": "x" })));
    assert!(!selects(&group, json!({ "a": "x", "b": "y" })));
    assert!(selects(&group, json!({ "c": "x", "e": "z" })));
    assert!(!selects(&group, json!({ "c": "x" })));

    let path = conditions("  group:\n    - $path: \"**/*.md\"\n    - a: string(required)\n");
    assert!(selects(&path, json!({ "a": "x" })), "the property alternative holds without a document");
    assert!(!selects(&path, json!({})), "a detached subject defeats `$path`");
}

#[test]
fn an_anonymous_nested_list_is_rejected_at_its_location() {
    for (body, at) in [
        ("  - - a: string(required)\n    - b: string(required)\n", "`match[0]`"),
        ("  - a: string(required)\n  - - b: string(required)\n", "`match[1]`"),
        ("  group:\n    - - a: string(required)\n", "`match.group[0]`"),
        ("  all:\n    - a: string(required)\n    - [b: string(required)]\n", "`match.all[1]`"),
    ] {
        let message = message(body);
        assert!(message.contains(at) && message.contains("`group:`"), "{body}: {message}");
    }
}

#[test]
fn the_vacuity_rule_judges_the_whole_expression() {
    // A guard beside a gate in a top-level list is covered by the AND.
    conditions("  - a: string(required)\n  - b: string\n");
    // A group with one guard alternative can be satisfied by absence.
    assert!(matches!(load("  group:\n    - a: string(required)\n    - b: string\n"), Err(SchemaError::TriggerVacuousMatch)));
    // ...unless a sibling top-level condition is a gate.
    conditions("  - c: string(required)\n  - group:\n      - a: string(required)\n      - b: string\n");
    // Only guards and `none`, anywhere in the list, are vacuous.
    assert!(matches!(load("  - a: string\n  - none:\n      - b: string(required)\n"), Err(SchemaError::TriggerVacuousMatch)));
    // A gate under `none` inside a group does not count.
    assert!(matches!(
        load("  group:\n    - a: string(required)\n    - none:\n        - b: string(required)\n"),
        Err(SchemaError::TriggerVacuousMatch)
    ));
    // `min-match` whose quota guards can fill.
    assert!(matches!(
        load("  min-match:\n    count: 1\n    of:\n      - group:\n          - a: string(required)\n      - b: string\n"),
        Err(SchemaError::TriggerVacuousMatch)
    ));
}

#[test]
fn any_is_rejected_naming_group_wherever_it_appears() {
    for (body, field) in [
        ("  any:\n    - a: string(required)\n", "match.any"),
        ("  - b: string(required)\n  - any:\n      - a: string(required)\n", "match[1].any"),
        ("  all:\n    - any:\n        - a: string(required)\n", "match.all[0].any"),
        ("  - b: string(required)\n  - none:\n      - any:\n          - a: string(required)\n", "match[1].none[0].any"),
    ] {
        match load(body) {
            Err(SchemaError::TriggerRenamed { field: found_field, found, replacement }) => {
                assert_eq!((found_field.as_str(), found.as_str(), replacement.as_str()), (field, "any:", "group:"), "{body}");
            }
            other => panic!("{body}: {other:?}"),
        }
    }
    let rendered = message("  any:\n    - a: string(required)\n");
    assert!(rendered.contains("use `group:`"), "{rendered}");
}
