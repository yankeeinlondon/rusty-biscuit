//! Authored flag origin ([`LifecycleParseOptions::authored_values`]): a host
//! parsing a schema-completed value names its authored source, so a filled
//! `no_error: false` is the omitted flag. Every case parses the completed
//! value with its source and the source alone, and the two must agree.

use serde_json::{Value, json};

use super::*;

fn options(authored_values: Option<&Value>) -> LifecycleParseOptions<'_> {
    LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry: super::super::registry::tests::agent_host(),
    }
}

fn parse(parsed: &Value, authored: Option<&Value>) -> Result<LifecycleConfig, LifecycleError> {
    parse_lifecycle_config_with(parsed, Path::new("doc.md"), &options(authored))
}

/// Parses `completed` with `authored` as its source, asserts the result
/// equals the parse of `authored` alone, and returns it.
fn agreed(completed: &Value, authored: &Value) -> Result<LifecycleConfig, String> {
    let with_source = parse(completed, Some(authored)).map_err(|error| error.to_string());
    let alone = parse(authored, None).map_err(|error| error.to_string());
    assert_eq!(with_source, alone, "completed {completed} with source {authored}");
    with_source
}

fn first_action(config: &LifecycleConfig, signal: LifecycleSignal) -> LifecycleAction {
    let mut found = Vec::new();
    actions::for_each_action(config.stack(signal), &mut |action| found.push(action.clone()));
    found.into_iter().next().expect("an action")
}

#[test]
fn filled_flags_beside_and_inside_a_long_form_are_not_two_placements() {
    let authored = json!({ "start": [{ "edit": { "file": "notes.md" } }] });
    let completed = json!({ "start": [{ "edit": { "file": "notes.md", "no_error": false, "create_new": false }, "no_error": false }] });
    let config = agreed(&completed, &authored).expect("one edit, no flag");
    assert!(!first_action(&config, LifecycleSignal::Start).no_error);
    // Read as authored, the filled flags would conflict.
    let unaware = parse(&completed, None).expect_err("two placements");
    assert!(unaware.to_string().contains("set both beside `edit` and inside it"), "{unaware}");
}

#[test]
fn an_authored_flag_in_either_place_survives_the_filled_one_in_the_other() {
    for (authored, completed) in [
        (
            json!({ "start": [{ "shell": { "command": "true" }, "no_error": true }] }),
            json!({ "start": [{ "shell": { "command": "true", "no_error": false }, "no_error": true }] }),
        ),
        (
            json!({ "start": [{ "shell": { "command": "true", "no_error": true } }] }),
            json!({ "start": [{ "shell": { "command": "true", "no_error": true }, "no_error": false }] }),
        ),
    ] {
        let config = agreed(&completed, &authored).expect("one authored placement");
        assert!(first_action(&config, LifecycleSignal::Start).no_error, "{authored}");
    }
}

#[test]
fn two_authored_placements_fail_even_when_equal_and_a_null_one_is_omitted() {
    for flag in [json!(true), json!(false)] {
        let authored = json!({ "start": [{ "shell": { "command": "true", "no_error": flag }, "no_error": flag }] });
        let error = agreed(&authored, &authored).expect_err("two placements");
        assert!(error.contains("set both beside `shell` and inside it"), "{error}");
    }
    // A written `null` is omitted; the completion then holds `false` there.
    let authored = json!({ "start": [{ "shell": { "command": "true", "no_error": null }, "no_error": true }] });
    let completed = json!({ "start": [{ "shell": { "command": "true", "no_error": false }, "no_error": true }] });
    assert!(first_action(&agreed(&completed, &authored).unwrap(), LifecycleSignal::Start).no_error);
}

#[test]
fn a_filled_flag_never_gives_set_a_flag_and_an_authored_one_even_null_is_refused() {
    let authored = json!({ "start": [{ "set": { "x": 1 } }] });
    let completed = json!({ "start": [{ "set": { "x": 1 }, "no_error": false }] });
    let config = agreed(&completed, &authored).expect("a set, no flag");
    let action = first_action(&config, LifecycleSignal::Start);
    assert!(matches!(action.kind, LifecycleActionKind::RuntimeSet(_)));
    assert!(!action.no_error);

    for flag in [json!(null), json!(false), json!(true)] {
        let authored = json!({ "start": [{ "set": { "x": 1 }, "no_error": flag }] });
        let completed = json!({ "start": [{ "set": { "x": 1 }, "no_error": false }] });
        let error = agreed(&completed, &authored).expect_err("`set` takes no flag");
        assert!(error.contains("`no_error` does not apply to `set`"), "{flag}: {error}");
    }
}

#[test]
fn a_filled_flag_beside_then_is_omitted_and_an_authored_one_is_refused() {
    let authored = json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "else": { "info": "b" } }] });
    let completed = json!({ "start": [{
        "when": "fm.x",
        "then": [{ "info": "a", "no_error": false }],
        "else": { "info": "b", "no_error": false },
        "no_error": false,
    }] });
    let config = agreed(&completed, &authored).expect("a conditional");
    assert!(matches!(config.stack(LifecycleSignal::Start)[0], LifecycleNode::Conditional(_)));

    let completed = json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a", "no_error": false }], "no_error": false }] });
    let authored = json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "no_error": false }] });
    let error = agreed(&completed, &authored).expect_err("an authored flag beside `then`");
    assert!(error.contains("cannot sit beside `then`/`else`"), "{error}");
    // `null` beside `then` is omitted.
    let authored = json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "no_error": null }] });
    agreed(&completed, &authored).expect("a null flag is no flag");
}

#[test]
fn a_filled_flag_alone_creates_no_action_and_changes_no_verb_count() {
    // An empty stack, item, or body the completion filled stays empty.
    agreed(&json!({ "start": { "no_error": false } }), &json!({ "start": {} })).expect("an empty stack");
    let error = agreed(&json!({ "start": [{ "no_error": false }] }), &json!({ "start": [{}] })).expect_err("empty item");
    assert!(error.contains("a stack item cannot be empty"), "{error}");
    let error = agreed(
        &json!({ "start": [{ "when": "fm.x", "then": { "no_error": false } }] }),
        &json!({ "start": [{ "when": "fm.x", "then": {} }] }),
    )
    .expect_err("an empty explicit body");
    assert!(error.contains("must contain at least one action"), "{error}");
    // A bundle keeps its own verbs.
    let config = agreed(
        &json!({ "start": { "info": "a", "warn": "b", "no_error": false } }),
        &json!({ "start": { "info": "a", "warn": "b" } }),
    )
    .unwrap();
    assert_eq!(config.stack(LifecycleSignal::Start).len(), 2);
}

#[test]
fn a_loop_block_filled_flag_is_no_concern_beside_a_gate_or_alone() {
    for (authored, completed) in [
        (json!({ "loop": { "max": 2 } }), json!({ "loop": { "max": 2, "no_error": false } })),
        (
            json!({ "loop": { "max": 2, "gate": [{ "info": "pass" }] } }),
            json!({ "loop": { "max": 2, "gate": [{ "info": "pass", "no_error": false }], "no_error": false } }),
        ),
    ] {
        agreed(&completed, &authored).expect("the loop parses");
    }
    // An authored flag beside a gate is still a loose key.
    let authored = json!({ "loop": { "max": 2, "gate": [{ "info": "pass" }], "no_error": true } });
    let error = agreed(&authored, &authored).expect_err("a flag beside the gate");
    assert!(error.contains("cannot sit beside"), "{error}");
}

#[test]
fn flags_convert_through_the_shared_engine() {
    for (flag, expected) in [(json!("yes"), true), (json!("no"), false), (json!(true), true)] {
        let authored = json!({ "start": [{ "info": "a", "no_error": flag.clone() }] });
        assert_eq!(first_action(&agreed(&authored, &authored).unwrap(), LifecycleSignal::Start).no_error, expected);
    }
    for flag in [json!(1), json!("{{ fm.flag }}"), json!([true])] {
        let authored = json!({ "start": [{ "info": "a", "no_error": flag.clone() }] });
        let error = agreed(&authored, &authored).expect_err("not a boolean");
        assert!(error.contains("`no_error` must be a boolean"), "{flag}: {error}");
    }
}

#[test]
fn single_action_and_task_stack_entries_take_the_authored_value_after_raw() {
    let registry = super::super::registry::tests::agent_host();
    let options = LifecycleParseOptions {
        is_known_effect: &|_| true,
        authored: None,
        authored_values: None,
        unknown_verbs: UnknownVerbPolicy::Reject,
        registry,
    };
    let authored = json!({ "set": { "x": 1 } });
    let completed = json!({ "set": { "x": 1 }, "no_error": false });
    let action = parse_single_action_with(
        LifecycleSignal::Start,
        &completed,
        Some(&authored),
        Path::new("seq.yaml"),
        "tasks[0].side_effect",
        None,
        &options,
    )
    .expect("a filled flag beside `set` is not authored");
    assert!(matches!(action.kind, LifecycleActionKind::RuntimeSet(_)));
    let error = parse_single_action_with(
        LifecycleSignal::Start,
        &completed,
        None,
        Path::new("seq.yaml"),
        "tasks[0].side_effect",
        None,
        &options,
    )
    .expect_err("read as authored, the flag meets `set`");
    assert!(error.to_string().contains("`no_error` does not apply to `set`"), "{error}");

    let authored = json!([{ "edit": { "file": "a.md" } }]);
    let completed = json!([{ "edit": { "file": "a.md", "no_error": false }, "no_error": false }]);
    let nodes = parse_task_action_stack_with(
        LifecycleSignal::Start,
        &completed,
        Some(&authored),
        Path::new("seq.yaml"),
        "tasks[0].setup",
        &AuthoredOrder::default(),
        &options,
    )
    .expect("one placement");
    assert_eq!(nodes.len(), 1);
}
