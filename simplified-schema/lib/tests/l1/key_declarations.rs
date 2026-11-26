//! `removed(<replacement>)` property declarations, the `$advise-unknown-keys`
//! opt-in, and the `did you mean` hints of unknown keys, judged through the
//! public `SimplifiedSchema` API.
//!
//! The default outcome of an invalid declaration is a schema grammar error;
//! the cells below list the shapes, and each value cell names its verdict.

use serde_json::{Value, json};
use simplified_schema::{
    SCHEMA_NEAR_MISS_KEY_CODE, SCHEMA_REMOVED_KEY_CODE, SimplifiedSchema, ValidationFailure, ValidationProblem,
    ValidationProblemCode,
};

#[track_caller]
fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

#[track_caller]
fn refused(text: &str) -> String {
    match SimplifiedSchema::try_from(text) {
        Ok(_) => panic!("schema must be refused:\n{text}"),
        Err(error) => error.to_string(),
    }
}

#[track_caller]
fn problems(schema: &SimplifiedSchema, value: &Value) -> Vec<ValidationProblem> {
    match schema.validate_strict(value) {
        Err(ValidationFailure::InvalidData { problems }) => problems,
        other => panic!("expected invalid data for {value}, got {other:?}"),
    }
}

#[track_caller]
fn sole_removed(schema: &SimplifiedSchema, value: &Value, parent: &str, key: &str) -> ValidationProblem {
    let found = problems(schema, value);
    assert_eq!(found.len(), 1, "{found:#?}");
    let problem = found.into_iter().next().unwrap();
    assert_eq!(problem.code, ValidationProblemCode::RemovedKey, "{problem:#?}");
    assert_eq!(problem.code.diagnostic_code(), SCHEMA_REMOVED_KEY_CODE);
    assert_eq!(problem.path, parent, "{problem:#?}");
    assert_eq!(problem.offending_property.as_deref(), Some(key), "{problem:#?}");
    assert_eq!(problem.description, None, "the owner's description never stands in for the removed key");
    problem
}

const ROOT_REMOVAL: &str = "$schema:\n  name: string\n  fail_fast: \"removed(set fail_fast on a group object)\"\n";

#[test]
fn an_absent_removed_key_is_accepted() {
    let schema = schema(ROOT_REMOVAL);
    assert!(schema.validate_strict(&json!({ "name": "x" })).is_ok());
    assert!(schema.validate_strict(&json!({})).is_ok());
}

#[test]
fn a_present_removed_key_is_refused_with_its_replacement_for_every_value() {
    let schema = schema(ROOT_REMOVAL);
    for value in [json!(true), json!(false), json!("yes"), json!(1), json!([]), json!({}), Value::Null] {
        let problem = sole_removed(&schema, &json!({ "fail_fast": value }), "", "fail_fast");
        assert_eq!(problem.hint.as_deref(), Some("set fail_fast on a group object"), "{value}");
        assert!(problem.message.contains("`fail_fast`"), "{}", problem.message);
        // Soft validation never converts a removed key into acceptance.
        assert!(schema.validate_soft(&json!({ "fail_fast": value })).is_err(), "{value}");
    }
}

#[test]
fn a_removed_key_is_located_at_its_key_token() {
    let text = "name: x\nfail_fast: true\n";
    let schema = schema(ROOT_REMOVAL);
    let value: Value = serde_yaml_ng::from_str(text).unwrap();
    let mut found = problems(&schema, &value);
    simplified_schema::validate::build_position_map(text).locate_all(&mut found);
    assert_eq!((found[0].line, found[0].column), (Some(2), Some(1)), "{found:#?}");
}

#[test]
fn a_replacement_keeps_backticks_semicolons_and_parentheses() {
    let schema = schema(
        "$schema:\n  setup: \"removed(use a preceding `side_effect:` task (or a `shell:` task); see docs)\"\n",
    );
    let problem = sole_removed(&schema, &json!({ "setup": "x" }), "", "setup");
    assert_eq!(
        problem.hint.as_deref(),
        Some("use a preceding `side_effect:` task (or a `shell:` task); see docs")
    );
}

#[test]
fn an_empty_replacement_is_refused() {
    for empty in ["removed()", "removed(   )"] {
        let message = refused(&format!("$schema:\n  old: \"{empty}\"\n"));
        assert!(message.contains("needs replacement advice"), "{message}");
    }
}

#[test]
fn removed_is_refused_on_a_pattern_key_and_in_a_types_table() {
    let message = refused("$schema:\n  \"<starting::x->\": \"removed(no)\"\n");
    assert!(message.contains("applies only to a named property"), "{message}");
    let message = refused("kind: schema\ntypes:\n  old: \"removed(no)\"\n");
    assert!(message.contains("`types` table"), "{message}");
}

#[test]
fn a_removal_overlapping_a_pattern_is_reported_once() {
    let schema = schema(
        "$schema:\n  fail_fast: \"removed(use groups)\"\n  \"<starting::fail>\": string\n  \"<string>\": unknown\n",
    );
    let problem = sole_removed(&schema, &json!({ "fail_fast": "text" }), "", "fail_fast");
    assert_eq!(problem.hint.as_deref(), Some("use groups"));
    // The pattern still admits its own keys.
    assert!(schema.validate_strict(&json!({ "fail_other": "text", "free": 1 })).is_ok());
}

#[test]
fn a_nested_and_a_union_reached_removal_names_its_owner() {
    let schema = schema(
        "kind: schema\ntypes:\n  task:\n    name: string\n    execution: \"removed(omit execution)\"\n    \"<string>\": unknown\n  step:\n    - string\n    - task\n$schema:\n  steps: \"step[]\"\n  group:\n    tasks: \"task[]\"\n    execution: \"removed(groups run concurrently)\"\n",
    );
    let problem = sole_removed(&schema, &json!({ "steps": ["a", { "name": "b", "execution": "serial" }] }), "/steps/1", "execution");
    assert_eq!(problem.hint.as_deref(), Some("omit execution"));
    let problem = sole_removed(&schema, &json!({ "group": { "execution": null } }), "/group", "execution");
    assert_eq!(problem.hint.as_deref(), Some("groups run concurrently"));
    let problem =
        sole_removed(&schema, &json!({ "group": { "tasks": [{ "execution": 1 }] } }), "/group/tasks/0", "execution");
    assert_eq!(problem.hint.as_deref(), Some("omit execution"));
}

#[test]
fn a_composed_object_keeps_its_operands_removals_and_a_later_member_wins() {
    let schema = schema(
        "kind: schema\ntypes:\n  base:\n    name: string\n    setup: \"removed(use a side_effect task)\"\n    teardown: \"removed(use finalize)\"\n  closed:\n    $extends: base\n  reopened:\n    $extends: base\n    teardown: string\n$schema:\n  a: closed\n  b: reopened\n",
    );
    let problem = sole_removed(&schema, &json!({ "a": { "setup": 1 } }), "/a", "setup");
    assert_eq!(problem.hint.as_deref(), Some("use a side_effect task"));
    // A closed composed object still refuses an undeclared key as unknown.
    let found = problems(&schema, &json!({ "a": { "nmae": "x" } }));
    assert_eq!(found[0].code, ValidationProblemCode::UnknownKey, "{found:#?}");
    assert_eq!(found[0].hint.as_deref(), Some("did you mean `name`?"));
    // The receiver's own declaration replaces the operand's removal.
    assert!(schema.validate_strict(&json!({ "b": { "teardown": "x" } })).is_ok());
    sole_removed(&schema, &json!({ "b": { "setup": 1 } }), "/b", "setup");
}

#[test]
fn a_removal_round_trips_through_export() {
    let original = schema(
        "$schema:\n  name: string\n  fail_fast: \"removed(set fail_fast on a group object)\"\n  \"<string>\": unknown\n  $advise-unknown-keys: [agent]\n",
    );
    let exported = original.to_simplified_yaml().expect("exports");
    assert!(exported.contains("removed(set fail_fast on a group object)"), "{exported}");
    let reread = schema(&exported);
    assert_eq!(reread.to_simplified_yaml().expect("exports again"), exported);
    sole_removed(&reread, &json!({ "fail_fast": true }), "", "fail_fast");
    assert_eq!(reread.key_advice(&json!({ "nmae": 1 })).len(), 1);
}

#[test]
fn removed_keys_lower_to_false_properties_with_owner_advice() {
    let json = schema(ROOT_REMOVAL).to_json_schema().expect("lowers");
    assert_eq!(json["properties"]["fail_fast"], Value::Bool(false));
    assert_eq!(json[simplified_schema::REMOVED_KEYS_ANNOTATION]["fail_fast"], "set fail_fast on a group object");
    assert!(json.get("required").is_none());
}

#[test]
fn a_closed_object_unknown_key_hint_follows_the_clean_ranking() {
    let schema = schema("$schema:\n  title: string\n  tags: \"string[]\"\n  $constraints: { closed: true }\n");
    let found = problems(&schema, &json!({ "titel": "x" }));
    assert_eq!(found[0].hint.as_deref(), Some("did you mean `title`?"), "{found:#?}");
    // No near candidate: no hint.
    let found = problems(&schema, &json!({ "zzzzzz": "x" }));
    assert_eq!(found[0].hint, None, "{found:#?}");
}

const OPEN_STEP: &str = "kind: schema\ntypes:\n  task:\n    name: string\n    prompt: string\n    model: string\n    \"<starting::x->\": unknown\n    \"<string>\": unknown\n    $advise-unknown-keys: [agent, modle]\n$schema:\n  steps: \"task[]\"\n";

#[test]
fn an_opted_in_open_object_advises_on_a_unique_near_miss_only() {
    let schema = schema(OPEN_STEP);
    let value = json!({ "steps": [{ "name": "a", "promt": "x.md", "x-extra": 1, "colour": "blue", "agent": "claude", "modle": "m" }] });
    // Advice never fails validation.
    let success = schema.validate_strict(&value).expect("open data is accepted");
    assert_eq!(success.key_advice.len(), 1, "{:#?}", success.key_advice);
    let advice = &success.key_advice[0];
    assert_eq!(advice.code, ValidationProblemCode::NearMissKey);
    assert_eq!(advice.code.diagnostic_code(), SCHEMA_NEAR_MISS_KEY_CODE);
    assert_eq!(advice.path, "/steps/0");
    assert_eq!(advice.offending_property.as_deref(), Some("promt"));
    assert_eq!(advice.hint.as_deref(), Some("did you mean `prompt`?"));
    assert_eq!(schema.key_advice(&value).len(), 1);
}

#[test]
fn an_ambiguous_near_miss_is_not_advised() {
    let schema = schema(
        "$schema:\n  note: string\n  nope: string\n  \"<string>\": unknown\n  $advise-unknown-keys: []\n",
    );
    assert!(schema.key_advice(&json!({ "nobe": 1 })).is_empty());
    assert_eq!(schema.key_advice(&json!({ "notee": 1 })).len(), 1);
}

#[test]
fn absent_opt_in_gives_no_advice() {
    let schema = schema("$schema:\n  prompt: string\n  \"<string>\": unknown\n");
    assert!(schema.key_advice(&json!({ "promt": 1 })).is_empty());
    assert!(schema.validate_strict(&json!({ "promt": 1 })).unwrap().key_advice.is_empty());
}

#[test]
fn the_opt_in_value_shapes_are_judged() {
    for (value, expect) in [
        ("null", "must be a list"),
        ("agent", "must be a list"),
        ("{ a: 1 }", "must be a list"),
        ("[agent, 3]", "item [1]"),
        ("[3]", "item [0]"),
        ("[\"\"]", "item [0]"),
    ] {
        let message = refused(&format!("$schema:\n  name: string\n  $advise-unknown-keys: {value}\n"));
        assert!(message.contains(expect), "{value}: {message}");
    }
    // The empty list opts in with no exclusions.
    let schema = schema("$schema:\n  prompt: string\n  \"<string>\": unknown\n  $advise-unknown-keys: []\n");
    assert_eq!(schema.key_advice(&json!({ "promt": 1 })).len(), 1);
}
