//! The lifecycle action grammar: ordered lists, dictionary bundles, nested
//! `when`/`then`/`else`, placement, `no_error` homes, removed spellings, and
//! authored paths through expansion (AC1–AC4, AC6, AC17; `with` placement for
//! AC5).

use crate::composition::LifecycleErrorInfoExt;
use crate::composition::LifecycleCatchProtocolExt;
use super::*;
use actions::{LifecycleConditional, LifecycleNode};

/// The six event keys; `loop` is exercised separately because it shares its
/// block with iteration controls.
const EVENTS: [&str; 6] = ["initialize", "start", "success", "blocked", "failure", "finalize"];

fn parse(frontmatter: serde_json::Value) -> Result<LifecycleConfig, CompositionError> {
    parse_lifecycle_config(&frontmatter, dummy_path())
}

fn parse_ok(frontmatter: serde_json::Value) -> LifecycleConfig {
    parse(frontmatter).expect("frontmatter parses")
}

fn parse_err(frontmatter: serde_json::Value) -> CompositionError {
    parse(frontmatter).expect_err("frontmatter is rejected")
}

fn signal_for(event: &str) -> LifecycleSignal {
    LifecycleSignal::ALL
        .into_iter()
        .find(|signal| signal.property_name() == event)
        .expect("known event")
}

/// The verb path of every top-level action, in execution order.
fn action_paths(nodes: &[LifecycleNode]) -> Vec<String> {
    nodes
        .iter()
        .map(|node| match node {
            LifecycleNode::Action(action) => action.path.to_string(),
            LifecycleNode::Conditional(conditional) => format!("{}?", conditional.path),
        })
        .collect()
}

fn conditional(node: &LifecycleNode) -> &LifecycleConditional {
    match node {
        LifecycleNode::Conditional(conditional) => conditional,
        LifecycleNode::Action(action) => panic!("expected a conditional, got {action:?}"),
    }
}

fn shape_property(error: &CompositionError) -> &str {
    match error {
        CompositionError::LifecycleStackInvalidShape { property, .. } => property,
        other => panic!("expected LifecycleStackInvalidShape, got {other:?}"),
    }
}

fn removed(error: CompositionError) -> (String, String, String) {
    match error {
        CompositionError::RemovedGrammar {
            path,
            removed,
            replacement,
            ..
        } => (path, removed, replacement),
        other => panic!("expected RemovedGrammar, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// AC1: every stack accepts a list and a root dictionary
// ---------------------------------------------------------------------------

#[test]
fn every_event_accepts_a_flat_action_list() {
    for event in EVENTS {
        let config = parse_ok(json!({ event: [{ "info": "a" }, { "warn": "b" }, "stop"] }));
        assert_eq!(
            action_paths(config.stack(signal_for(event))),
            [format!("{event}[0].info"), format!("{event}[1].warn"), format!("{event}[2].stop")],
            "{event}"
        );
    }
}

#[test]
fn every_event_accepts_a_root_dictionary_in_canonical_order() {
    for event in EVENTS {
        // `serde_json` maps iterate in key order (effect, message, say,
        // stderr, stop), which is not the canonical order.
        let config = parse_ok(json!({ event: {
            "stop": null,
            "say": "spoken",
            "effect": "confirmation",
            "message": "sent",
            "stderr": "line",
        }}));
        assert_eq!(
            action_paths(config.stack(signal_for(event))),
            [
                format!("{event}.stderr"),
                format!("{event}.message"),
                format!("{event}.effect"),
                format!("{event}.say"),
                format!("{event}.stop"),
            ],
            "{event}"
        );
    }
}

#[test]
fn a_bundle_expands_set_communication_shell_effects_then_the_directive() {
    let config = parse_ok(json!({ "start": {
        "stop": null,
        "shell": "make",
        "set": { "phase": "built" },
        "ensure_dir": "out",
        "append_line": ["log.txt", "x"],
        "speak": "hi",
        "stdout": "out",
        "notify": "title",
    }}));
    assert_eq!(
        action_paths(config.stack(LifecycleSignal::Start)),
        [
            "start.set",
            "start.stdout",
            "start.notify",
            "start.speak",
            "start.shell",
            "start.append_line",
            "start.ensure_dir",
            "start.stop",
        ]
    );
}

#[test]
fn a_root_dictionary_never_carries_when() {
    let error = parse_err(json!({ "start": { "when": "fm.go", "info": "x" } }));
    assert_eq!(shape_property(&error), "start.when");
}

#[test]
fn a_single_verb_root_dictionary_may_use_long_form() {
    let config = parse_ok(json!({ "start": { "shell": { "command": "git status" } } }));
    let LifecycleNode::Action(action) = &config.stack(LifecycleSignal::Start)[0] else {
        panic!("expected an action");
    };
    assert_eq!(action.path.to_string(), "start.shell");
    assert!(matches!(action.kind, LifecycleActionKind::Shell(_)));
}

#[test]
fn absent_null_and_empty_stacks_are_all_empty() {
    for value in [json!(null), json!([]), json!({})] {
        let config = parse_ok(json!({ "start": value.clone(), "finalize": value }));
        assert!(config.is_empty());
    }
    assert!(parse_ok(json!({ "title": "no events" })).is_empty());
}

#[test]
fn a_stack_of_the_wrong_type_is_rejected_not_read_as_empty() {
    for value in [json!("info"), json!(3), json!(true)] {
        let error = parse_err(json!({ "start": value }));
        assert_eq!(shape_property(&error), "start");
    }
}

#[test]
fn the_loop_gate_is_a_stack() {
    let config = parse_ok(json!({ "loop": {
        "while": "fm.n < 3",
        "action": "increment(n)",
        "gate": [
            { "effect": "confirmation" },
            { "when": "fm.n == 3", "then": [{ "warn": "cap" }] },
            { "info": "done {{ fm.n }}" },
        ],
    }}));
    assert_eq!(
        action_paths(config.stack(LifecycleSignal::Loop)),
        ["loop.gate[0].effect", "loop.gate[1]?", "loop.gate[2].info"]
    );
}

#[test]
fn loose_loop_verbs_are_a_root_dictionary_beside_iteration_controls() {
    let config = parse_ok(json!({ "loop": {
        "while": "fm.n < 3",
        "action": "increment(n)",
        "max": 5,
        "info": "finished",
        "stderr": "gate",
    }}));
    assert_eq!(
        action_paths(config.stack(LifecycleSignal::Loop)),
        ["loop.stderr", "loop.info"]
    );
}

#[test]
fn loose_loop_verbs_cannot_mix_with_gate() {
    let error = parse_err(json!({ "loop": {
        "while": "true",
        "info": "x",
        "gate": [{ "warn": "y" }],
    }}));
    assert_eq!(shape_property(&error), "loop.info");
}

#[test]
fn when_at_the_loop_root_is_rejected() {
    let error = parse_err(json!({ "loop": {
        "while": "true",
        "when": "fm.n > 1",
        "info": "x",
    }}));
    assert_eq!(shape_property(&error), "loop.when");
}

#[test]
fn task_stacks_accept_a_list_or_a_dictionary_rooted_at_the_task_property() {
    let list = parse_task_action_stack(
        LifecycleSignal::Start,
        &json!([{ "info": "a" }, { "when": "fm.x", "then": [{ "shell": "make" }] }]),
        dummy_path(),
        "tasks[0].setup",
    )
    .unwrap();
    assert_eq!(action_paths(&list), ["tasks[0].setup[0].info", "tasks[0].setup[1]?"]);
    let LifecycleNode::Action(shell) = &conditional(&list[1]).then[0] else {
        panic!("expected an action");
    };
    assert_eq!(shell.path.to_string(), "tasks[0].setup[1].then[0].shell");

    let dictionary = parse_task_action_stack(
        LifecycleSignal::Finalize,
        &json!({ "shell": "cleanup", "info": "bye" }),
        dummy_path(),
        "tasks[1].teardown",
    )
    .unwrap();
    assert_eq!(
        action_paths(&dictionary),
        ["tasks[1].teardown.info", "tasks[1].teardown.shell"]
    );
}

// ---------------------------------------------------------------------------
// Branch shape
// ---------------------------------------------------------------------------

#[test]
fn then_and_else_parse_into_ordered_bodies() {
    let config = parse_ok(json!({ "start": [
        {
            "when": "ctx.season == 'summer'",
            "then": [{ "info": "summer" }, { "shell": "run-summer" }],
            "else": [{ "info": "not summer" }],
        },
        { "message": "always" },
    ]}));
    let stack = config.stack(LifecycleSignal::Start);
    assert_eq!(action_paths(stack), ["start[0]?", "start[1].message"]);
    let branch = conditional(&stack[0]);
    assert_eq!(action_paths(&branch.then), ["start[0].then[0].info", "start[0].then[1].shell"]);
    assert_eq!(action_paths(&branch.otherwise), ["start[0].else[0].info"]);
}

#[test]
fn a_guarded_bundle_becomes_the_true_branch_in_canonical_order() {
    let config = parse_ok(json!({ "start": [{
        "when": "ctx.season == 'summer'",
        "say": "water?",
        "message": "heat",
    }]}));
    let branch = conditional(&config.stack(LifecycleSignal::Start)[0]);
    assert_eq!(action_paths(&branch.then), ["start[0].message", "start[0].say"]);
    assert!(branch.otherwise.is_empty());
}

#[test]
fn a_body_may_be_a_dictionary_with_its_own_no_error() {
    let config = parse_ok(json!({ "failure": [{
        "when": "err.category == 'cap'",
        "then": { "warn": "cap", "proxy": "@prompts/other.md", "no_error": true },
        "else": [{ "warn": "{{ err.msg }}" }],
    }]}));
    let branch = conditional(&config.stack(LifecycleSignal::Failure)[0]);
    assert_eq!(action_paths(&branch.then), ["failure[0].then.warn", "failure[0].then.proxy"]);
    for node in &branch.then {
        let LifecycleNode::Action(action) = node else { panic!() };
        assert!(action.no_error, "{}", action.path);
    }
}

#[test]
fn explicitly_empty_branches_are_rejected() {
    for branch in ["then", "else"] {
        for empty in [json!(null), json!([]), json!({})] {
            let mut item = serde_json::Map::new();
            item.insert("when".into(), json!("fm.x"));
            item.insert("then".into(), json!([{ "info": "t" }]));
            item.insert(branch.into(), empty);
            let error = parse_err(json!({ "start": [item] }));
            assert_eq!(shape_property(&error), format!("start[0].{branch}"));
        }
    }
}

#[test]
fn orphan_branches_guard_only_and_flag_only_items_are_rejected() {
    let cases = [
        (json!({ "then": [{ "info": "x" }] }), "start[0].then"),
        (json!({ "else": [{ "info": "x" }] }), "start[0].else"),
        (json!({ "when": "fm.x" }), "start[0]"),
        (json!({ "when": "fm.x", "no_error": true }), "start[0]"),
        (json!({ "no_error": true }), "start[0]"),
        (json!({}), "start[0]"),
        (json!({ "when": "fm.x", "else": [{ "info": "x" }], "info": "y" }), "start[0].else"),
    ];
    for (item, path) in cases {
        let error = parse_err(json!({ "start": [item.clone()] }));
        assert_eq!(shape_property(&error), path, "{item}");
    }
}

#[test]
fn then_cannot_sit_beside_sibling_verbs_or_no_error() {
    let error = parse_err(json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "warn": "b" }] }));
    assert_eq!(shape_property(&error), "start[0].warn");
    let error = parse_err(json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "no_error": true }] }));
    assert_eq!(shape_property(&error), "start[0].no_error");
}

#[test]
fn a_dictionary_body_never_carries_when() {
    let error = parse_err(json!({ "start": [{
        "when": "fm.x",
        "then": { "when": "fm.y", "info": "a" },
    }]}));
    assert_eq!(shape_property(&error), "start[0].then.when");
}

#[test]
fn a_when_that_is_not_a_string_or_not_an_expression_is_rejected() {
    let error = parse_err(json!({ "start": [{ "when": true, "info": "a" }] }));
    assert_eq!(shape_property(&error), "start[0].when");
    let error = parse_err(json!({ "start": [{ "when": "fm.x ==", "info": "a" }] }));
    let CompositionError::LifecycleWhenExpressionInvalid { property, .. } = error else {
        panic!("expected LifecycleWhenExpressionInvalid, got {error:?}");
    };
    assert_eq!(property, "start[0].when");
}

// ---------------------------------------------------------------------------
// AC2: conditional nesting
// ---------------------------------------------------------------------------

/// A `then` chain `depth` conditionals deep, ending in one action.
fn nested_then(depth: usize) -> serde_json::Value {
    let mut body = json!([{ "info": "deepest" }]);
    for _ in 0..depth {
        body = json!([{ "when": "true", "then": body }]);
    }
    body
}

#[test]
fn conditionals_nest_five_levels_deep() {
    let config = parse_ok(json!({ "failure": nested_then(5) }));
    let mut nodes = config.stack(LifecycleSignal::Failure);
    for _ in 0..5 {
        nodes = &conditional(&nodes[0]).then;
    }
    assert_eq!(action_paths(nodes), ["failure[0].then[0].then[0].then[0].then[0].then[0].info"]);
}

#[test]
fn a_sixth_level_is_a_typed_error_naming_its_path() {
    let error = parse_err(json!({ "failure": nested_then(6) }));
    assert_eq!(
        shape_property(&error),
        "failure[0].then[0].then[0].then[0].then[0].then[0].when"
    );
}

// Claudine's schema accepts `{}` as an empty bundle, so the parser is the only
// layer that rejects an empty stack item.
#[test]
fn an_empty_stack_item_is_a_typed_error_naming_its_path() {
    for (event, value, path) in [
        ("start", json!([{}]), "start[0]"),
        ("initialize", json!([{}]), "initialize[0]"),
        ("failure", json!([{ "when": "fm.x", "then": [{}] }]), "failure[0].then[0]"),
    ] {
        let error = parse_err(json!({ event: value }));
        assert_eq!(shape_property(&error), path, "{event}");
        let CompositionError::LifecycleStackInvalidShape { message, .. } = &error else { unreachable!() };
        assert!(message.contains("cannot be empty"), "{event}: {message}");
    }
}

#[test]
fn an_else_if_chain_counts_each_nested_when_and_siblings_do_not() {
    // Five levels through `else`, with sibling guards at every level.
    let mut body = json!([{ "warn": "last" }]);
    for _ in 0..5 {
        body = json!([
            { "when": "false", "then": [{ "info": "sibling" }] },
            { "when": "fm.x", "then": [{ "retry": 3 }], "else": body },
        ]);
    }
    parse_ok(json!({ "failure": body.clone() }));

    let deeper = json!([{ "when": "fm.y", "then": body }]);
    let error = parse_err(json!({ "failure": deeper }));
    assert_eq!(
        shape_property(&error),
        "failure[0].then[1].else[1].else[1].else[1].else[0].when"
    );
}

// ---------------------------------------------------------------------------
// AC3: flow-control placement is list-local
// ---------------------------------------------------------------------------

fn order_property(error: &CompositionError) -> &str {
    match error {
        CompositionError::LifecycleActionOrder { property, .. } => property,
        other => panic!("expected LifecycleActionOrder, got {other:?}"),
    }
}

#[test]
fn an_action_after_an_unconditional_directive_is_rejected_in_every_list() {
    let error = parse_err(json!({ "success": ["stop", { "info": "never" }] }));
    assert_eq!(order_property(&error), "success[0].stop");

    let error = parse_err(json!({ "failure": [{
        "when": "fm.x",
        "then": [{ "retry": 3 }, { "info": "never" }],
    }]}));
    assert_eq!(order_property(&error), "failure[0].then[0].retry");

    let error = parse_err(json!({ "failure": [{
        "when": "fm.x",
        "then": [{ "info": "a" }],
        "else": [{ "error": "bad" }, { "warn": "never" }],
    }]}));
    assert_eq!(order_property(&error), "failure[0].else[0].error");

    // A bundle holding a directive is unconditional too.
    let error = parse_err(json!({ "start": [{ "info": "a", "stop": null }, { "warn": "never" }] }));
    assert_eq!(order_property(&error), "start[0].stop");

    let error = parse_err(json!({ "loop": { "while": "true", "gate": ["stop", { "info": "x" }] } }));
    assert_eq!(order_property(&error), "loop.gate[0].stop");
}

#[test]
fn a_when_item_never_counts_as_unconditional() {
    parse_ok(json!({ "failure": [
        {
            "when": "err.is_throttled",
            "then": [{ "info": "throttled" }, { "retry": 3 }],
            "else": [{ "warn": "other" }, { "error": "fatal" }],
        },
        { "message": "failure handled" },
        { "when": "fm.x", "stop": null },
        { "info": "still reachable" },
    ]}));
}

#[test]
fn the_same_directive_in_sibling_branches_is_legal() {
    parse_ok(json!({ "failure": [{
        "when": "fm.x",
        "then": [{ "retry": 2 }],
        "else": [{ "retry": 5 }],
    }]}));
}

#[test]
fn two_directives_in_one_dictionary_are_rejected() {
    let error = parse_err(json!({ "start": [{ "stop": null, "error": "x" }] }));
    let CompositionError::LifecycleMultipleLifecycleActions { property, .. } = error else {
        panic!("expected LifecycleMultipleLifecycleActions, got {error:?}");
    };
    assert_eq!(property, "start[0]");
}

#[test]
fn skip_stays_initialize_only_inside_branches() {
    parse_ok(json!({ "initialize": [{ "when": "fm.x", "then": ["skip"] }] }));
    let error = parse_err(json!({ "start": [{ "when": "fm.x", "then": [{ "info": "a" }], "else": ["skip"] }] }));
    let CompositionError::LifecycleActionPlacement { property, action, .. } = error else {
        panic!("expected LifecycleActionPlacement, got {error:?}");
    };
    assert_eq!((property.as_str(), action.as_str()), ("start[0].else[0].skip", "skip"));
}

// ---------------------------------------------------------------------------
// AC4: the two homes of `no_error`
// ---------------------------------------------------------------------------

fn only_action(config: &LifecycleConfig, signal: LifecycleSignal) -> &LifecycleAction {
    match config.stack(signal) {
        [LifecycleNode::Action(action)] => action,
        other => panic!("expected one action, got {other:?}"),
    }
}

#[test]
fn no_error_is_accepted_as_a_long_form_parameter() {
    let config = parse_ok(json!({ "success": [{ "shell": { "command": "post.sh", "no_error": true } }] }));
    assert!(only_action(&config, LifecycleSignal::Success).no_error);
}

#[test]
fn no_error_is_accepted_beside_the_verbs_of_a_bundle_but_never_reaches_set() {
    let config = parse_ok(json!({ "success": [{
        "when": "ctx.notify_team",
        "message": "done",
        "shell": "post.sh",
        "set": { "posted": true },
        "no_error": true,
    }]}));
    let branch = conditional(&config.stack(LifecycleSignal::Success)[0]);
    let flags: Vec<(String, bool)> = branch
        .then
        .iter()
        .map(|node| match node {
            LifecycleNode::Action(action) => (action.path.to_string(), action.no_error),
            LifecycleNode::Conditional(_) => panic!("bundle expands to actions"),
        })
        .collect();
    assert_eq!(
        flags,
        [
            ("success[0].set".to_string(), false),
            ("success[0].message".to_string(), true),
            ("success[0].shell".to_string(), true),
        ]
    );
}

#[test]
fn no_error_defaults_to_false() {
    let config = parse_ok(json!({ "success": [{ "shell": "post.sh" }] }));
    assert!(!only_action(&config, LifecycleSignal::Success).no_error);
}

#[test]
fn no_error_beside_a_lone_set_is_rejected() {
    let error = parse_err(json!({ "start": [{ "set": { "a": 1 }, "no_error": true }] }));
    assert_eq!(shape_property(&error), "start[0].no_error");
}

#[test]
fn set_has_no_long_form_no_error() {
    // A mapping under `set` is always mutation data: `no_error` is a key.
    let config = parse_ok(json!({ "start": [{ "set": { "no_error": true } }] }));
    let action = only_action(&config, LifecycleSignal::Start);
    let LifecycleActionKind::RuntimeSet(set) = &action.kind else { panic!() };
    assert!(set.get("no_error").is_some());
    assert!(!action.no_error);
}

#[test]
fn inner_and_sibling_no_error_are_ambiguous_even_when_they_agree() {
    let error = parse_err(json!({ "start": [{
        "shell": { "command": "x", "no_error": true },
        "no_error": true,
    }]}));
    assert_eq!(shape_property(&error), "start[0].shell.no_error");
}

#[test]
fn no_error_must_be_a_yaml_boolean_in_both_homes() {
    let error = parse_err(json!({ "start": [{ "shell": "x", "no_error": "true" }] }));
    assert_eq!(shape_property(&error), "start[0].no_error");
    let error = parse_err(json!({ "start": [{ "shell": { "command": "x", "no_error": 1 } }] }));
    let CompositionError::LifecycleActionInvalidLongForm { property, .. } = error else {
        panic!("expected LifecycleActionInvalidLongForm, got {error:?}");
    };
    assert_eq!(property, "start[0].shell");
}

// ---------------------------------------------------------------------------
// AC17: bundles, long forms, aliases, unknown keys
// ---------------------------------------------------------------------------

#[test]
fn several_sibling_verbs_use_short_form_only() {
    let error = parse_err(json!({ "start": [{ "info": "x", "shell": { "command": "y" } }] }));
    assert_eq!(shape_property(&error), "start[0].shell");
}

#[test]
fn say_and_speak_cannot_share_a_dictionary() {
    let error = parse_err(json!({ "start": [{ "say": "a", "speak": "b" }] }));
    let CompositionError::LifecycleSayConflict(property) = error else {
        panic!("expected LifecycleSayConflict, got {error:?}");
    };
    assert_eq!(property, "start[0]");
}

#[test]
fn speak_keeps_its_authored_key_in_the_path() {
    let config = parse_ok(json!({ "start": [{ "speak": "hello" }] }));
    let action = only_action(&config, LifecycleSignal::Start);
    assert_eq!(action.path.to_string(), "start[0].speak");
    let LifecycleActionKind::Communication(comm) = &action.kind else { panic!() };
    assert_eq!(comm.channel, CommunicationChannel::Speak);
}

#[test]
fn unknown_item_keys_are_unknown_verbs_not_discarded() {
    let error = parse_err(json!({ "start": [{ "info": "a", "mesage": "b" }] }));
    let CompositionError::LifecycleUnknownVerb { property, verb, rewrite, .. } = error else {
        panic!("expected LifecycleUnknownVerb, got {error:?}");
    };
    assert_eq!((property.as_str(), verb.as_str()), ("start[0].mesage", "mesage"));
    assert!(rewrite.contains("message"), "{rewrite}");
}

#[test]
fn unknown_long_form_parameters_are_rejected() {
    let error = parse_err(json!({ "start": [{ "shell": { "command": "x", "cwd": "/tmp" } }] }));
    let CompositionError::LifecycleActionInvalidLongForm { property, message, .. } = error else {
        panic!("expected LifecycleActionInvalidLongForm, got {error:?}");
    };
    assert_eq!(property, "start[0].shell");
    assert!(message.contains("cwd"), "{message}");
}

#[test]
fn communication_long_form_takes_text() {
    let config = parse_ok(json!({ "failure": [{ "say": { "text": "the run failed", "no_error": true } }] }));
    let action = only_action(&config, LifecycleSignal::Failure);
    assert!(action.no_error);
    let LifecycleActionKind::Communication(comm) = &action.kind else { panic!() };
    assert_eq!(comm.text, Expr::StringLiteral("the run failed".to_string()));
}

#[test]
fn short_form_arity_is_checked_with_the_authored_path() {
    let error = parse_err(json!({ "failure": [
        { "when": "fm.x", "then": [{ "info": "a" }], "else": [{ "info": "b" }, { "shell": ["a", "b"] }] },
    ]}));
    let CompositionError::LifecycleWrongArity { property, verb, .. } = error else {
        panic!("expected LifecycleWrongArity, got {error:?}");
    };
    assert_eq!((property.as_str(), verb.as_str()), ("failure[0].else[1].shell", "shell"));
}

#[test]
fn object_arguments_still_pass_through_whole_value_interpolation() {
    let config = parse_ok(json!({ "success": [{ "http_post": ["https://example.test", "{{ fm.payload }}"] }] }));
    let LifecycleActionKind::SideEffect(effect) = &only_action(&config, LifecycleSignal::Success).kind else {
        panic!("expected a side effect");
    };
    assert_eq!(effect.args.len(), 2);
    assert!(!matches!(effect.args[1], Expr::StringLiteral(_)));

    let error = parse_err(json!({ "success": [{ "http_post": { "url": "u", "body": { "a": 1 } } }] }));
    assert!(matches!(
        error,
        CompositionError::LifecycleObjectDataThroughInterpolationParameter { .. }
    ));
}

#[test]
fn an_unknown_literal_effect_is_rejected_in_any_branch_and_a_template_is_deferred() {
    let error = parse_err(json!({ "success": [
        { "when": "fm.x", "then": [{ "info": "a" }], "else": [{ "effect": "no-such-effect" }] },
    ]}));
    let CompositionError::LifecycleUnknownEffect(property, name) = error else {
        panic!("expected LifecycleUnknownEffect, got {error:?}");
    };
    assert_eq!((property.as_str(), name.as_str()), ("success[0].else[0].effect", "no-such-effect"));
    parse_ok(json!({ "success": [{ "effect": "{{ fm.sound }}" }] }));
}

#[test]
fn initialize_refuses_shells_in_either_branch() {
    let error = parse_err(json!({ "initialize": [{
        "when": "fm.x",
        "then": [{ "info": "a" }],
        "else": [{ "when": "fm.y", "then": [{ "shell": "make" }] }],
    }]}));
    let CompositionError::LifecycleActionPlacement { property, event, .. } = error else {
        panic!("expected LifecycleActionPlacement, got {error:?}");
    };
    assert_eq!((property.as_str(), event.as_str()), ("initialize[0].else[0].then[0].shell", "initialize"));
}

#[test]
fn initialize_refuses_a_shell_valued_set_in_a_dead_branch() {
    let error = parse_err(json!({ "initialize": [{
        "when": "false",
        "then": [{ "set": { "sha": "$(git rev-parse HEAD)" } }],
    }]}));
    let CompositionError::LifecycleActionPlacement { property, .. } = error else {
        panic!("expected LifecycleActionPlacement, got {error:?}");
    };
    assert_eq!(property, "initialize[0].then[0].set");
}

#[test]
fn both_branches_contribute_shells_to_approval() {
    let config = parse_ok(json!({ "failure": [
        {
            "when": "fm.x",
            "then": [{ "shell": "first" }],
            "else": [{ "info": "b" }, { "shell": { "command": "second", "on_error": "third" } }],
        },
    ]}));
    assert_eq!(
        collect_lifecycle_shell_commands(&config),
        [
            ("first".to_string(), "failure[0].then[0].shell.command".to_string()),
            ("second".to_string(), "failure[0].else[1].shell.command".to_string()),
            ("third".to_string(), "failure[0].else[1].shell.on_error".to_string()),
        ]
    );
}

// ---------------------------------------------------------------------------
// AC5 (parse side): `with` placement
// ---------------------------------------------------------------------------

#[test]
fn with_is_accepted_on_retry_and_proxy() {
    let config = parse_ok(json!({ "failure": [{ "retry": {
        "max_attempts": 3,
        "backoff": "exponential",
        "with": { "reason": "{{ err.msg }}" },
    }}]}));
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Retry { with, .. }) =
        &only_action(&config, LifecycleSignal::Failure).kind
    else {
        panic!("expected retry");
    };
    assert!(with.get("reason").is_some());

    let config = parse_ok(json!({ "failure": [{ "proxy": { "target": "@other.md", "with": { "a": 1 } } }] }));
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Proxy { with, .. }) =
        &only_action(&config, LifecycleSignal::Failure).kind
    else {
        panic!("expected proxy");
    };
    assert_eq!(with.len(), 1);
}

#[test]
fn with_is_rejected_on_every_other_verb_resume_included() {
    for (verb, action) in [
        ("resume", json!({ "message": "go on", "with": { "a": 1 } })),
        ("shell", json!({ "command": "x", "with": { "a": 1 } })),
        ("error", json!({ "reason": "x", "with": { "a": 1 } })),
    ] {
        let error = parse_err(json!({ "failure": [{ verb: action }] }));
        let CompositionError::LifecycleWithUnsupported { property, verb: named, .. } = error else {
            panic!("expected LifecycleWithUnsupported for {verb}, got {error:?}");
        };
        assert_eq!((property, named), (format!("failure[0].{verb}"), verb.to_string()));
    }
}

#[test]
fn a_retry_overlay_has_the_proxy_shape_rules() {
    let error = parse_err(json!({ "failure": [{ "retry": { "with": "{{ fm.payload }}" } }] }));
    let CompositionError::LifecycleWithWholeMapping { property, path, .. } = error else {
        panic!("expected LifecycleWithWholeMapping, got {error:?}");
    };
    assert_eq!(format!("{property}.{path}"), "failure[0].retry.with");

    let error = parse_err(json!({ "failure": [{ "retry": { "with": { "{{ k }}": 1 } } }] }));
    assert!(matches!(error, CompositionError::LifecycleWithDynamicKey { .. }));

    let error = parse_err(json!({ "failure": [{ "retry": { "with": [1] } }] }));
    assert!(matches!(error, CompositionError::LifecycleWithNotMapping { .. }));
}

// ---------------------------------------------------------------------------
// AC6 (parse side): removed spellings
// ---------------------------------------------------------------------------

#[test]
fn the_stack_key_is_removed_grammar_in_every_stack() {
    for event in EVENTS {
        let (path, key, replacement) =
            removed(parse_err(json!({ event: { "stack": [{ "info": "x" }] } })));
        assert_eq!((path, key.as_str()), (format!("{event}.stack"), "stack"));
        assert!(replacement.contains("ordered list"), "{replacement}");
    }
    let (path, ..) = removed(parse_err(json!({ "loop": { "while": "true", "stack": [] } })));
    assert_eq!(path, "loop.stack");
    let (path, ..) = removed(parse_err(json!({ "start": [{ "when": "fm.x", "then": { "stack": [] } }] })));
    assert_eq!(path, "start[0].then.stack");
}

#[test]
fn the_action_aggregator_and_discriminator_are_removed_grammar() {
    let (path, key, replacement) = removed(parse_err(json!({ "start": [{ "action": [{ "info": "x" }] }] })));
    assert_eq!((path.as_str(), key.as_str()), ("start[0].action", "action"));
    assert!(replacement.contains("one verb per item"), "{replacement}");

    let (path, _, replacement) =
        removed(parse_err(json!({ "start": [{ "when": "fm.x", "action": "stop" }] })));
    assert_eq!(path, "start[0].action");
    assert!(replacement.contains("then"), "{replacement}");

    let (path, _, replacement) =
        removed(parse_err(json!({ "success": [{ "action": "shell", "command": "make" }] })));
    assert_eq!(path, "success[0].action");
    assert!(replacement.contains("- shell:"), "{replacement}");

    let error = parse_task_action_stack(
        LifecycleSignal::Start,
        &json!([{ "action": "stop" }]),
        dummy_path(),
        "tasks[0].setup",
    )
    .unwrap_err();
    assert_eq!(removed(error).0, "tasks[0].setup[0].action");
}

#[test]
fn say_first_is_removed_grammar() {
    let (path, key, replacement) = removed(parse_err(json!({ "start": { "say_first": "hi" } })));
    assert_eq!((path.as_str(), key.as_str()), ("start.say_first", "say_first"));
    assert!(replacement.contains("order"), "{replacement}");
    let (path, ..) = removed(parse_err(json!({ "loop": { "while": "true", "say_first": "hi" } })));
    assert_eq!(path, "loop.say_first");
}

#[test]
fn communication_message_sound_and_route_parameters_are_removed_grammar() {
    for (verb, param) in [("say", "message"), ("effect", "sound"), ("message", "route"), ("info", "message")] {
        let mut params = serde_json::Map::new();
        params.insert("text".into(), json!("x"));
        params.insert(param.into(), json!("y"));
        let (path, key, replacement) = removed(parse_err(json!({ "start": [{ verb: params }] })));
        assert_eq!(path, format!("start[0].{verb}.{param}"));
        assert_eq!(key, format!("{verb}.{param}"));
        let hint = if param == "route" { "remove `route`" } else { "text" };
        assert!(replacement.contains(hint), "{replacement}");
    }
}

#[test]
fn resume_message_stays_a_valid_parameter() {
    let config = parse_ok(json!({ "failure": [{ "resume": { "message": "continue", "max_attempts": 2 } }] }));
    assert!(matches!(
        only_action(&config, LifecycleSignal::Failure).kind,
        LifecycleActionKind::LifecycleControl(LifecycleControlAction::Resume { .. })
    ));
}

#[test]
fn removed_keys_win_over_every_other_shape_error() {
    // `stack` beside an unknown key and a root `when` still reports `stack`.
    let (path, ..) = removed(parse_err(json!({ "start": { "stack": [], "when": "x", "bogus": 1 } })));
    assert_eq!(path, "start.stack");
}

#[test]
fn cancelled_parses_cleanup_actions_and_preserves_authored_paths() {
    let config = parse_ok(json!({"cancelled": [
        {"info": "cancelled"}, {"set": {"done": true}},
        {"shell": "echo cleanup"}, "stop"
    ]}));
    assert!(!config.is_empty(), "cancelled must be retained, not ignored");
    assert_eq!(action_paths(config.stack(signal_for("cancelled"))), [
        "cancelled[0].info", "cancelled[1].set", "cancelled[2].shell", "cancelled[3].stop"
    ]);
}

#[test]
fn cancelled_rejects_null_instead_of_silently_opting_in() {
    let error = parse_err(json!({"cancelled": null}));
    assert_eq!(shape_property(&error), "cancelled");
}

#[test]
fn cancelled_rejects_recovery_and_other_control_actions() {
    for action in [json!("skip"), json!("error"), json!("retry"),
        json!({"resume": "continue"}), json!({"proxy": "next.md"}),
        json!({"defer": "1s"}), json!({"prep": "next.md"}), json!("break")] {
        let error = parse_err(json!({"cancelled": [action.clone()]}));
        assert!(matches!(error,
            CompositionError::LifecycleActionPlacement { ref event, .. } if event == "cancelled"
        ), "{action}: {error:?}");
    }
}

#[test]
fn cancelled_accepts_expression_families_and_validates_err_reads() {
    let frontmatter = json!({"cancelled": [
        {"ensure_file": ["cleanup.txt", "done"]},
        {"number": "42"},
        {"when": "err != null", "then": [{"info": "{{ err }}"}],
            "else": [{"shell": "echo fallback"}]}
    ]});
    let config = parse_ok(frontmatter.clone());
    assert_eq!(config.stack(signal_for("cancelled")).len(), 3);
    validate_no_err_in_no_error_events(&frontmatter, &config, dummy_path()).unwrap();
    validate_no_nested_spans_in_literals(&frontmatter, &config, dummy_path()).unwrap();
    assert_eq!(collect_lifecycle_shell_commands(&config),
        vec![("echo fallback".to_string(), "cancelled[2].else[0].shell.command".to_string())]);
}

#[test]
fn cancelled_empty_stacks_and_invalid_shapes_are_distinct() {
    for value in [json!([]), json!({})] {
        assert!(parse_ok(json!({"cancelled": value})).stack(signal_for("cancelled")).is_empty());
    }
    for value in [json!(true), json!(4), json!("cleanup"), json!([1]), json!([{"info": "ok"}, 1])] {
        assert!(matches!(parse_err(json!({"cancelled": value})),
            CompositionError::LifecycleStackInvalidShape { .. }));
    }
}

#[test]
fn cancelled_terminal_reserves_finalize_and_carries_the_cause() {
    let error = crate::composition::LifecycleErrorInfo::from_action_failure("interrupted", "cancelled".to_string());
    let mut protocol = runtime::LifecycleCatchProtocol::new(
        LifecycleSignal::Cancelled,
        runtime::LifecycleCatchState {
            terminal_slot: Some(LifecycleSignal::Cancelled), finalize_emitted: false,
        },
        Some(error.clone()),
        executor::LifecycleEventOutcome::default(),
    );
    let cleanup = protocol.next_step().unwrap();
    assert_eq!(cleanup.signal, LifecycleSignal::Finalize);
    assert_eq!(cleanup.error.as_ref(), Some(&error));
    assert!(protocol.record(LifecycleSignal::Finalize, executor::LifecycleEventOutcome::default()));
    assert!(protocol.next_step().is_none());
}

#[test]
fn an_empty_cancelled_stack_retains_the_cancellation_opt_in() {
    assert!(parse_ok(json!({})).is_empty());
    for value in [json!([]), json!({})] {
        let config = parse_ok(json!({"cancelled": value}));
        assert!(config.cancelled_declared);
        assert!(!config.is_empty(), "an explicitly authored cancellation opt-in must survive an empty action stack");
        assert!(!config.clone().is_empty(), "the downstream runtime receives the opt-in when configuration is cloned");
        assert!(config.stack(signal_for("cancelled")).is_empty());
    }
}
