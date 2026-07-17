//! action shape control lifecycle tests.

use super::*;
use super::actions::ProxyWithValue;
use darkmatter::markdown::compose::expression::{EvaluationLookup, evaluate};
use serde_json::Value;
use std::collections::HashMap;

#[test]
fn rejects_short_form_say_action() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": "say('hello world')"}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
            assert_eq!(raw, "say('hello world')");
            assert_eq!(rewrite, "say: \"hello world\"");
        }
        other => panic!("expected LifecycleShortFormRemoved, got: {other:?}"),
    }
}

#[test]
fn positional_scalar_value_is_taken_literally() {
    // A positional scalar value is literal text by default — `ctx.repo` is
    // the text, not the context expression. Use a whole-value `{{ … }}`
    // span (resolved at event time) to interpolate a value.
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "ctx.repo"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("ctx.repo".to_string()));
}

#[test]
fn parses_when_condition_with_stack() {
    let fm = json!({
        "start": {
            "stack": [
                {
                    "when": "env.AGENT == 'claude'",
                    "action": {"say": "using claude"}
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].when.is_some());
}

#[test]
fn parses_multiple_actions_per_stack_item() {
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": [
                        {"say": "first"},
                        {"info": "second"}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack[0].actions.len(), 2);
}

#[test]
fn parses_stop_short_form() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": "stop"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 1);
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_skip_in_initialize() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": "skip"}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_retry_with_count_in_blocked() {
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"retry": 3}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Blocked)
        .expect("blocked stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_proxy_with_file_arg_in_initialize() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": "@fallback.md"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert!(stack[0].actions[0].is_lifecycle_control());
}

#[test]
fn parses_shell_long_form_with_on_error_and_no_error() {
    // Long-form shell action: `command`, `on_error`, `no_error` live
    // inside the explicit `{ action: shell, ... }` object.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": {
                        "action": "shell",
                        "command": "git fetch --all",
                        "on_error": "fetch failed",
                        "no_error": true
                    }
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let action = &stack[0].actions[0];
    assert!(action.no_error);
}

#[test]
fn parses_side_effect_long_form() {
    // Side-effect long form: `file`, `prop`, `value` live inside the
    // explicit `{ action: set_frontmatter, ... }` object.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": {
                        "action": "set_frontmatter",
                        "file": "@spec.md",
                        "prop": "status",
                        "value": "in-progress"
                    }
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let _ = config.stack(LifecycleSignal::Start).expect("start stack");
}

#[test]
fn parses_side_effect_short_form() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"ensure_file": "@out/log.md"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let _ = config.stack(LifecycleSignal::Start).expect("start stack");
}

#[test]
fn rejects_skip_outside_initialize() {
    let fm = json!({
        "start": {"stack": [{"action": "skip"}]}
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionPlacement {
            action, event, ..
        } => {
            assert_eq!(action, "skip");
            assert_eq!(event, "start");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }
}

#[test]
fn flow_control_is_universal_across_events() {
    // Flow control reacts to state, not just errors, so `error`/`retry`/
    // `resume`/`requeue`/`proxy` parse in every event (only `skip` is
    // placement-restricted, to `initialize`). E.g. a `success` stack may
    // `resume` because an expected artifact was not produced.
    let cases: [(&str, serde_json::Value); 6] = [
        ("start", json!({"proxy": "@other.md"})),
        ("start", json!({"retry": null})),
        (
            "success",
            json!({"resume": "the file abc.md was never written; create it"}),
        ),
        ("blocked", json!({"resume": "please"})),
        ("initialize", json!({"defer": "5m"})),
        ("success", json!({"retry": 2})),
    ];
    for (event, action) in cases {
        let fm = json!({ event: {"stack": [{"action": action}]} });
        parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("`{action}` in `{event}` should parse, got: {e:?}"));
    }
    // `loop` carries iteration controls; a `requeue` there parses too.
    let loop_fm = json!({ "loop": {"while": "true", "stack": [{"action": {"defer": "5m"}}]} });
    parse_lifecycle_config(&loop_fm, dummy_path())
        .unwrap_or_else(|e| panic!("`requeue` in `loop` should parse, got: {e:?}"));
}

#[test]
fn accepts_recovery_actions_in_finalize() {
    // `finalize` is the optional-error terminal event and a last-chance
    // recovery surface, so retry/resume/requeue/proxy all parse there
    // (parity with the `failure` event).
    for action in [
        json!({"retry": 1}),
        json!({"resume": "finish the task"}),
        json!({"defer": "5m"}),
        json!({"proxy": "@other.md"}),
    ] {
        let fm = json!({
            "finalize": {"stack": [{"when": "err", "action": action}]}
        });
        parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("finalize `{action}` should parse, got: {e:?}"));
    }
}

#[test]
fn rejects_multiple_lifecycle_actions_in_one_item() {
    let fm = json!({
        "blocked": {
            "stack": [
                {"action": ["stop", "skip"]}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleMultipleLifecycleActions { .. }
    ));
}

#[test]
fn rejects_lifecycle_action_not_last() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": ["stop", {"say": "unreachable"}]}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleActionOrder { .. }
    ));
}

#[test]
fn accepts_lifecycle_action_as_last() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": [{"say": "one"}, "stop"]}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 2);
    assert!(!stack[0].actions[0].is_lifecycle_control());
    assert!(stack[0].actions[1].is_lifecycle_control());
}

#[test]
fn control_checks_fire_identically_for_key_value_form() {
    // The cardinality, ordering, and placement checks operate on the parsed
    // typed `LifecycleControlAction` — independent of whether the author
    // wrote the control positional (`{"action": "skip"}` / `{"stop": null}`)
    // or key/value (`{"action": {"action": "stop"}}`). The positional-form
    // tests above already pin the behavior; this pins the same diagnostics
    // for the key/value form so the two forms cannot drift.

    // Placement: a key/value `skip` outside `initialize` is the same
    // LifecycleActionPlacement error the positional `{"action": "skip"}`
    // trips in `rejects_skip_outside_initialize`.
    let fm = json!({
        "start": {"stack": [{"action": {"action": "skip"}}]}
    });
    match parse_lifecycle_config(&fm, dummy_path()).unwrap_err() {
        CompositionError::LifecycleActionPlacement { action, event, .. } => {
            assert_eq!(action, "skip");
            assert_eq!(event, "start");
        }
        other => panic!("expected LifecycleActionPlacement, got: {other:?}"),
    }

    // Cardinality: two key/value control actions in one item trip
    // LifecycleMultipleLifecycleActions (parity with the positional
    // `["stop", "skip"]` case).
    let fm = json!({
        "blocked": {"stack": [{"action": [{"action": "stop"}, {"action": "skip"}]}]}
    });
    assert!(matches!(
        parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
        CompositionError::LifecycleMultipleLifecycleActions { .. }
    ));

    // Ordering: a key/value control action before a non-control action trips
    // LifecycleActionOrder (parity with `["stop", {"say": ...}]`).
    let fm = json!({
        "initialize": {"stack": [{"action": [
            {"action": "stop"},
            {"action": "say", "message": "unreachable"}
        ]}]}
    });
    assert!(matches!(
        parse_lifecycle_config(&fm, dummy_path()).unwrap_err(),
        CompositionError::LifecycleActionOrder { .. }
    ));

    // Positive parity: a key/value control action as the LAST item is
    // accepted, exactly like the positional form.
    let fm = json!({
        "initialize": {"stack": [{"action": [
            {"action": "say", "message": "one"},
            {"action": "stop"}
        ]}]}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    assert_eq!(stack[0].actions.len(), 2);
    assert!(!stack[0].actions[0].is_lifecycle_control());
    assert!(stack[0].actions[1].is_lifecycle_control());
}

#[test]
fn positional_scalar_value_is_literal_text() {
    // Positional scalar values are literal text by default — `using codex`
    // is the text, not an expression. Commas and colons inside are part of
    // the message.
    let cases: [( &str, serde_json::Value, &str); 4] = [
        ("say", json!({"say": "using codex"}), "using codex"),
        (
            "warn",
            json!({"warn": "phase 6, too big"}),
            "phase 6, too big",
        ),
        (
            "error",
            json!({"error": "invalid phase: 6"}),
            "invalid phase: 6",
        ),
        (
            "effect",
            json!({"effect": "crowd-applause"}),
            "crowd-applause",
        ),
    ];
    for (verb, action, expected) in cases {
        let fm = json!({ "blocked": { "stack": [{"action": action}] } });
        let config = parse_lifecycle_config(&fm, dummy_path())
            .unwrap_or_else(|e| panic!("`{verb}` positional scalar should parse, got: {e:?}"));
        let stack = config.stack(LifecycleSignal::Blocked).expect("blocked stack");
        let message = match &stack[0].actions[0].kind {
            LifecycleActionKind::Communication(c) => &c.message,
            LifecycleActionKind::LifecycleControl(LifecycleControlAction::Error {
                reason: Some(r),
            }) => r,
            other => panic!("unexpected action kind for `{verb}`: {other:?}"),
        };
        assert_eq!(message, &Expr::StringLiteral(expected.to_string()), "{verb}");
    }
}

#[test]
fn rejects_missing_closing_paren() {
    let fm = json!({
        "start": {
            "stack": [{"action": "say('hi'"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleShortFormRemoved { .. }
    ));
}

#[test]
fn rejects_retry_with_too_many_args() {
    let fm = json!({
        "blocked": {
            "stack": [{"action": {"retry": [3, 4]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleWrongArity { .. }
    ));
}

#[test]
fn rejects_proxy_missing_target() {
    // `proxy` requires a `target` parameter; a null positional value is
    // wrong arity.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": null}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleWrongArity { .. }
    ));
}

#[test]
fn rejects_stack_item_missing_action_key() {
    let fm = json!({
        "start": {
            "stack": [{"when": "true"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

#[test]
fn rejects_unknown_stack_item_key() {
    // A scalar `action` value cannot carry sibling parameter keys; the
    // `bogus` key is rejected as an invalid stack-item shape.
    let fm = json!({
        "start": {
            "stack": [{"action": "stop", "bogus": true}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

#[test]
fn rejects_stack_item_that_is_not_an_object() {
    let fm = json!({
        "start": {
            "stack": ["stop"]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(matches!(
        err,
        CompositionError::LifecycleStackInvalidShape { .. }
    ));
}

// -- positional action parser (Phase 4) ----------------------------------

#[test]
fn parses_positional_communication_scalar() {
    let fm = json!({
        "success": {
            "stack": [
                {"action": {"message": "hello"}},
                {"action": {"effect": "crowd-applause"}},
                {"action": {"stderr": "an error"}},
                {"action": {"success": "it worked"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack.len(), 4);
    for item in stack {
        assert!(matches!(item.actions[0].kind, LifecycleActionKind::Communication(_)));
    }
}

#[test]
fn parses_positional_shell_scalar() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"shell": "git status"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(matches!(stack[0].actions[0].kind, LifecycleActionKind::Shell(_)));
}

#[test]
fn parses_positional_side_effect_array() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"set_frontmatter": ["s.md", "status", "ready"]}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let action = &stack[0].actions[0];
    let LifecycleActionKind::SideEffect(se) = &action.kind else {
        panic!("expected side-effect action, got {action:?}");
    };
    assert_eq!(se.verb, "set_frontmatter");
    assert_eq!(se.args.len(), 3);
}

#[test]
fn parses_positional_optional_tail_side_effect() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"ensure_file": ["out/log.md"]}},
                {"action": {"ensure_file": ["out/log.md", "# log"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);
    for item in stack {
        let LifecycleActionKind::SideEffect(se) = &item.actions[0].kind else {
            panic!("expected side-effect action");
        };
        assert_eq!(se.verb, "ensure_file");
    }
}

#[test]
fn parses_positional_control_verbs() {
    let fm = json!({
        "initialize": {
            "stack": [
                {"action": {"stop": null}},
                {"action": {"stop": []}},
                {"action": {"error": "reason"}},
                {"action": {"retry": 3}},
                {"action": {"proxy": "@other.md"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Initialize).expect("init stack");
    assert_eq!(stack.len(), 5);
    for item in stack {
        assert!(item.actions[0].is_lifecycle_control());
    }
}

#[test]
fn parses_positional_expression_function_variadic() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"and": ["true", "true", "false"]}},
                {"action": {"or": ["a", "b"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);
    for item in stack {
        assert!(matches!(
            item.actions[0].kind,
            LifecycleActionKind::ExpressionFunction(_)
        ));
    }
}

#[test]
fn parses_positional_expression_function_concrete() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"length": "{{ items }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "length");
    assert_eq!(ef.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_bracket_optional() {
    // `number(x, [default])` — the bracketed param is optional, so the
    // one-argument form is valid arity.
    let fm = json!({
        "start": {
            "stack": [{"action": {"number": "{{ value }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "number");
    assert_eq!(ef.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_overload_one_arg() {
    // Overloaded functions accept their shortest (one-argument) form: the
    // longer overload's extra parameters are optional.
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"frontmatter": "state.md"}},
                {"action": {"link": "state.md"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 2);

    let LifecycleActionKind::ExpressionFunction(frontmatter) = &stack[0].actions[0].kind else {
        panic!("expected frontmatter expression-function action");
    };
    assert_eq!(frontmatter.function, "frontmatter");
    assert_eq!(frontmatter.args.len(), 1);

    let LifecycleActionKind::ExpressionFunction(link) = &stack[1].actions[0].kind else {
        panic!("expected link expression-function action");
    };
    assert_eq!(link.function, "link");
    assert_eq!(link.args.len(), 1);
}

#[test]
fn parses_positional_expression_function_happy_path() {
    // Confirm the existing fixed-arity expression functions still parse.
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"length": "{{ items }}"}},
                {"action": {"contains": ["{{ haystack }}", "{{ needle }}"]}},
                {"action": {"and": ["true", "true"]}},
                {"action": {"or": ["a", "b"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack.len(), 4);
    for item in stack {
        assert!(matches!(
            item.actions[0].kind,
            LifecycleActionKind::ExpressionFunction(_)
        ));
    }
}

#[test]
fn parses_positional_typed_arguments() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"set_frontmatter": ["s.md", "ready", "{{ true }}"]}},
                {"action": {"merge_frontmatter": ["s.md", "{{ payload }}"]}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");

    let LifecycleActionKind::SideEffect(set) = &stack[0].actions[0].kind else {
        panic!("expected set_frontmatter side-effect");
    };
    assert_eq!(set.args[2], Expr::BoolLiteral(true));

    let LifecycleActionKind::SideEffect(merge) = &stack[1].actions[0].kind else {
        panic!("expected merge_frontmatter side-effect");
    };
    assert!(matches!(merge.args[1], Expr::Variable(_)));
}

#[test]
fn parses_positional_action_object_value() {
    // `action: { success: "..." }` is the single-object positional form.
    let fm = json!({
        "success": {
            "stack": [{"action": {"success": "it worked"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack[0].actions.len(), 1);
    assert!(matches!(
        stack[0].actions[0].kind,
        LifecycleActionKind::Communication(_)
    ));
}

#[test]
fn rejects_positional_wrong_arity_side_effect() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"set_frontmatter": ["s.md"]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_wrong_arity_communication() {
    let fm = json!({
        "success": {
            "stack": [{"action": {"message": ["a", "b"]}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_bare_proxy_as_wrong_arity() {
    // `proxy` requires a target; a null/empty-array value is wrong arity,
    // not a short-form issue.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": null}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { .. }),
        "expected wrong-arity error, got: {err:?}"
    );
}

#[test]
fn rejects_positional_unknown_verb() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"sucess": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, .. } => {
            assert_eq!(verb, "sucess");
        }
        other => panic!("expected LifecycleUnknownVerb, got: {other:?}"),
    }
}

#[test]
fn rejects_positional_object_value() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"merge_frontmatter": {"status": "ready"}}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::LifecycleObjectDataThroughInterpolationPositional { .. }
        ),
        "expected object-data-through-interpolation error, got: {err:?}"
    );
}

#[test]
fn rejects_ambiguous_multi_key_action_object() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"message": "hi", "route": "team"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
        "expected ambiguous error, got: {err:?}"
    );
}

#[test]
fn positional_and_key_value_action_object_coexist_in_array() {
    // The motivating shape from the spec: positional and key/value actions
    // in the same stack array.
    let fm = json!({
        "success": {
            "stack": [
                {
                    "when": "true",
                    "action": [
                        {"success": "it worked"},
                        {"set_frontmatter": ["s.md", "status", "done"]},
                        {"action": "shell", "command": "git push"}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Success).expect("success stack");
    assert_eq!(stack[0].actions.len(), 3);
    assert!(matches!(
        stack[0].actions[0].kind,
        LifecycleActionKind::Communication(_)
    ));
    assert!(matches!(
        stack[0].actions[1].kind,
        LifecycleActionKind::SideEffect(_)
    ));
    assert!(matches!(stack[0].actions[2].kind, LifecycleActionKind::Shell(_)));
}

#[test]
fn parses_stdout_field_on_event_block() {
    let fm = json!({
        "start": {"stdout": "hello"}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.start.as_ref().unwrap().stdout.as_deref(),
        Some("hello")
    );
}

#[test]
fn parses_stdout_short_form_action() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"stdout": "hello"}}]
        }
    });
    // `stdout: ...` is a recognized positional communication action;
    // parsing succeeds and produces a single-item stack.
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(config.stack(LifecycleSignal::Start).unwrap().len(), 1);
}

#[test]
fn parses_stdout_field_on_loop_block() {
    // A top-level `loop.stdout` is extracted as a loop lifecycle concern,
    // alongside the iteration controls. The `while` key keeps the loop
    // block otherwise valid.
    let fm = json!({
        "loop": {"while": "true", "stdout": "hello"}
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert_eq!(
        config.loop_concerns.as_ref().unwrap().stdout.as_deref(),
        Some("hello")
    );
}

#[test]
fn legacy_top_level_only_prompts_still_parse() {
    // Legacy prompts that only configure the four original top-level
    // events (`start`, `success`, `blocked`, `failure`) continue to parse
    // and expose those events through `LifecycleConfig::get` exactly as
    // before the seven-event model was introduced.
    let fm = json!({
        "start":   { "stderr": "starting" },
        "success": { "stderr": "done" },
        "blocked": { "stderr": "blocked" },
        "failure": { "stderr": "failed" }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    assert!(config.initialize.is_none());
    assert!(config.finalize.is_none());
    assert!(config.loop_concerns.is_none());
    assert!(config.stacks.start.is_none());
    assert!(!config.is_empty());
    // `get` continues to work for the four legacy signals.
    for s in [
        LifecycleSignal::Start,
        LifecycleSignal::Success,
        LifecycleSignal::Blocked,
        LifecycleSignal::Failure,
    ] {
        assert!(config.get(s).is_some(), "expected {s:?} to be configured");
    }
}

// =====================================================================
// Phase 5: positional-and-key-value action validation checkpoint
// =====================================================================

#[test]
fn short_form_rejection_rewrites_to_positional() {
    // Removed `verb(args)` short form is rejected with a did-you-mean
    // positional rewrite.
    let cases: [(&str, serde_json::Value, &str); 3] = [
        ("success", json!({"success": "x"}), "success: \"x\""),
        ("shell", json!({"shell": "git push"}), "shell: \"git push\""),
        (
            "set_frontmatter",
            json!({"set_frontmatter": ["a", "b", "c"]}),
            "set_frontmatter: [\"a\", \"b\", \"c\"]",
        ),
    ];
    for (verb, action, expected_rewrite) in cases {
        let short_form = format!("{verb}({})", match verb {
            "success" => "\"x\"".to_string(),
            "shell" => "git push".to_string(),
            "set_frontmatter" => "'a','b','c'".to_string(),
            _ => unreachable!(),
        });
        let fm = json!({
            "start": {
                "stack": [{"action": short_form.clone()}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleShortFormRemoved { raw, rewrite, .. } => {
                assert_eq!(raw, short_form, "{verb}");
                assert_eq!(rewrite, expected_rewrite, "{verb}");
            }
            other => panic!("expected LifecycleShortFormRemoved for {verb}, got: {other:?}"),
        }

        // The positional rewrite itself parses cleanly.
        let fm = json!({
            "start": {
                "stack": [{"action": action}]
            }
        });
        assert!(
            parse_lifecycle_config(&fm, dummy_path()).is_ok(),
            "{verb} positional rewrite should parse"
        );
    }
}

#[test]
fn bare_stop_accepted_bare_proxy_rejected_wrong_arity() {
    // Zero-arg positional: bare `stop` is accepted.
    let fm = json!({
        "initialize": {
            "stack": [{"action": "stop"}]
        }
    });
    assert!(parse_lifecycle_config(&fm, dummy_path()).is_ok());

    // `proxy` requires a target; a bare verb is wrong arity.
    let fm = json!({
        "initialize": {
            "stack": [{"action": "proxy"}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleWrongArity { ref verb, .. } if verb == "proxy"),
        "expected wrong-arity for bare proxy, got: {err:?}"
    );
}

#[test]
fn key_value_literal_default_vs_whole_value_interpolation() {
    // Key/value literal default: a plain string parameter is a literal.
    let fm = json!({
        "start": {
            "stack": [{"action": {"action": "message", "message": "ctx.area"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("ctx.area".to_string()));

    // Whole-value interpolation resolves the expression at event time.
    let fm = json!({
        "start": {
            "stack": [{"action": {"action": "message", "message": "{{ ctx.area }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::Variable("ctx.area".to_string()));
}

#[test]
fn full_disambiguation_table_for_positional_and_key_value() {
    // Same verb as positional single-key object and as explicit key/value.
    let positional = json!({"start": {"stack": [{"action": {"success": "it worked"}}]}});
    let key_value = json!({
        "start": {
            "stack": [{"action": {"action": "success", "message": "it worked"}}]
        }
    });
    for fm in [&positional, &key_value] {
        let config = parse_lifecycle_config(fm, dummy_path()).unwrap();
        let stack = config.stack(LifecycleSignal::Start).expect("start stack");
        assert!(matches!(
            stack[0].actions[0].kind,
            LifecycleActionKind::Communication(_)
        ));
    }

    // Multi-key object without an `action` key is ambiguous.
    let fm = json!({
        "start": {
            "stack": [{"action": {"message": "hi", "route": "team"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    assert!(
        matches!(err, CompositionError::LifecycleStackAmbiguous { .. }),
        "expected ambiguous error, got: {err:?}"
    );
}

#[test]
fn predicate_exception_when_evaluates_expression_scalar_stays_literal() {
    // `when` is always a boolean expression.
    let fm = json!({
        "start": {
            "stack": [
                {"when": "true", "action": {"say": "true"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].when.is_some());

    // The positional scalar `"true"` is literal text, not a bool.
    let LifecycleActionKind::Communication(comm) = &stack[0].actions[0].kind else {
        panic!("expected communication action");
    };
    assert_eq!(comm.message, Expr::StringLiteral("true".to_string()));
}

#[test]
fn known_verb_validation_for_typoed_positional_and_key_value() {
    // Typoed positional verb gets a did-you-mean suggestion.
    let fm = json!({
        "success": {
            "stack": [{"action": {"sucess": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
            assert_eq!(verb, "sucess");
            assert!(rewrite.contains("success"), "got: {rewrite}");
        }
        other => panic!("expected LifecycleUnknownVerb for positional typo, got: {other:?}"),
    }

    // Typoed key/value verb gets the same suggestion.
    let fm = json!({
        "success": {
            "stack": [{"action": {"action": "sucess", "message": "it worked"}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleUnknownVerb { verb, rewrite, .. } => {
            assert_eq!(verb, "sucess");
            assert!(rewrite.contains("success"), "got: {rewrite}");
        }
        other => panic!("expected LifecycleUnknownVerb for key/value typo, got: {other:?}"),
    }
}

#[test]
fn expression_function_actions_positional_key_value_and_variadic_rejection() {
    // Positional expression-function action.
    let fm = json!({
        "start": {
            "stack": [{"action": {"length": "{{ items }}"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "length");
    assert_eq!(ef.args.len(), 1);
    assert_eq!(ef.args[0], Expr::Variable("items".to_string()));

    // Key/value expression-function action with concrete named parameters.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "contains",
                    "haystack": "{{ haystack }}",
                    "needle": "needle"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "contains");
    assert_eq!(ef.args.len(), 2);

    // Variadic expression functions reject key/value form.
    for verb in ["and", "or"] {
        let fm = json!({
            "start": {
                "stack": [{"action": {"action": verb, "a": "true", "b": "false"}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        assert!(
            matches!(
                err,
                CompositionError::LifecycleExpressionFunctionKeyValueUnsupported {
                    verb: ref v, ..
                } if v == verb
            ),
            "{verb} key/value should be rejected, got: {err:?}"
        );
    }
}

#[test]
fn key_value_expression_function_rejects_missing_required_param() {
    // `contains(haystack, needle)` — both required. Supplying only
    // `haystack` must fail at parse time, naming the missing `needle`.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "contains",
                    "haystack": "{{ haystack }}"
                }
            }]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionInvalidLongForm {
            action, message, ..
        } => {
            assert_eq!(action, "contains");
            assert!(
                message.contains("needle"),
                "message should name the missing `needle` param, got: {message}"
            );
        }
        other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
    }
}

#[test]
fn key_value_side_effect_rejects_missing_required_params() {
    // `set_frontmatter(file, prop, value)` — all required. Supplying only
    // `file` must fail at parse time, naming both missing params.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "set_frontmatter",
                    "file": "@state.md"
                }
            }]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleActionInvalidLongForm {
            action, message, ..
        } => {
            assert_eq!(action, "set_frontmatter");
            assert!(
                message.contains("prop") && message.contains("value"),
                "message should name both missing params, got: {message}"
            );
        }
        other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
    }
}

#[test]
fn key_value_omitting_optional_tail_param_parses() {
    // `frontmatter(file, [prop])` (expression function) — `prop` is an
    // optional tail param, so the `file`-only key/value form is valid.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "frontmatter",
                    "file": "@spec.md"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::ExpressionFunction(ef) = &stack[0].actions[0].kind else {
        panic!("expected expression-function action");
    };
    assert_eq!(ef.function, "frontmatter");
    assert_eq!(ef.args.len(), 1);

    // `ensure_file(file, [content])` (side effect) — `content` is optional,
    // so the `file`-only key/value form is valid.
    let fm = json!({
        "start": {
            "stack": [{
                "action": {
                    "action": "ensure_file",
                    "file": "@out/log.md"
                }
            }]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    let LifecycleActionKind::SideEffect(se) = &stack[0].actions[0].kind else {
        panic!("expected side-effect action");
    };
    assert_eq!(se.verb, "ensure_file");
    assert_eq!(se.args.len(), 1);
}

#[test]
fn collect_lifecycle_shell_commands_extracts_literal_commands() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"action": "shell", "command": "git fetch --all"}},
                {"action": {"say": "not a shell command"}}
            ]
        },
        "failure": {
            "stack": [
                {"action": {"action": "shell", "command": "git reset --hard", "on_error": "cleanup failed"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    let command_strings: Vec<&str> = commands.iter().map(|(c, _)| c.as_str()).collect();
    assert!(
        command_strings.contains(&"git fetch --all"),
        "expected git fetch, got: {command_strings:?}"
    );
    assert!(
        command_strings.contains(&"git reset --hard"),
        "expected git reset, got: {command_strings:?}"
    );
    assert!(
        command_strings.contains(&"cleanup failed"),
        "expected on_error command, got: {command_strings:?}"
    );
}

#[test]
fn collect_lifecycle_shell_commands_empty_when_no_shells() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "hello"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    assert!(commands.is_empty(), "got: {commands:?}");
}

#[test]
fn collect_lifecycle_shell_commands_carries_property_path() {
    let fm = json!({
        "start": {
            "stack": [
                {"action": {"action": "shell", "command": "echo hi"}}
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let commands = collect_lifecycle_shell_commands(&config);
    assert_eq!(commands.len(), 1);
    let (_, property) = &commands[0];
    assert!(
        property.contains("start.stack[0]") && property.contains(".command"),
        "expected property path, got: {property}"
    );
}

// -- no_error on every action category --------------------------------

#[test]
fn no_error_flag_is_accepted_on_every_action_category() {
    // The universal `no_error: true` flag must be accepted on every
    // action category: communication, shell, side-effect, and
    // expression-function.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": [
                        {"action": "say", "message": "hi", "no_error": true},
                        {"action": "shell", "command": "echo hi", "no_error": true},
                        {"action": "set_frontmatter", "file": "@a.md", "prop": "x", "value": "y", "no_error": true},
                        {"action": "length", "x": "hello", "no_error": true}
                    ]
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert_eq!(stack[0].actions.len(), 4);
    for action in &stack[0].actions {
        assert!(action.no_error, "no_error should be true for {:?}", action.kind);
    }
}

#[test]
fn no_error_on_scalar_form_threads_to_every_category() {
    // Scalar form: `no_error` is a sibling key alongside a bare-verb
    // zero-arg `action` value.
    let fm = json!({
        "start": {
            "stack": [
                {
                    "action": "stop",
                    "no_error": true
                }
            ]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(stack[0].actions[0].no_error);
}

#[test]
fn no_error_defaults_to_false() {
    let fm = json!({
        "start": {
            "stack": [{"action": {"say": "hi"}}]
        }
    });
    let config = parse_lifecycle_config(&fm, dummy_path()).unwrap();
    let stack = config.stack(LifecycleSignal::Start).expect("start stack");
    assert!(!stack[0].actions[0].no_error);
}

// =====================================================================
// Phase 5: runtime state machine
// =====================================================================

struct MapLookup(HashMap<String, Value>);

impl EvaluationLookup for MapLookup {
    fn get(&self, path: &str) -> Option<Value> {
        self.0.get(path).cloned()
    }
}

struct EmptyLookup;

impl EvaluationLookup for EmptyLookup {
    fn get(&self, _path: &str) -> Option<Value> {
        None
    }
}

#[test]
fn action_value_to_expr_plain_literal() {
    let expr = action_value_to_expr(&json!("hello world")).unwrap();
    assert_eq!(expr, Expr::StringLiteral("hello world".into()));
}

#[test]
fn action_value_to_expr_multi_span_interpolation_stays_literal() {
    let expr = action_value_to_expr(&json!("before {{ x }} after")).unwrap();
    assert_eq!(expr, Expr::StringLiteral("before {{ x }} after".into()));
}

#[test]
fn action_value_to_expr_whole_value_bool() {
    let expr = action_value_to_expr(&json!("{{ true }}")).unwrap();
    assert_eq!(expr, Expr::BoolLiteral(true));
}

#[test]
fn action_value_to_expr_whole_value_number() {
    let expr = action_value_to_expr(&json!("{{ 3 }}")).unwrap();
    assert_eq!(expr, Expr::NumberLiteral(3.0));
}

#[test]
fn action_value_to_expr_whole_value_with_surrounding_whitespace() {
    let expr = action_value_to_expr(&json!("  {{ true }}  ")).unwrap();
    assert_eq!(expr, Expr::BoolLiteral(true));
}

#[test]
fn action_value_to_expr_whole_value_null() {
    let expr = action_value_to_expr(&json!("{{ null }}")).unwrap();
    assert_eq!(evaluate(&expr, &EmptyLookup).unwrap(), Value::Null);
}

#[test]
fn action_value_to_expr_whole_value_object_passthrough() {
    let payload = json!({ "status": "ready", "count": 7 });
    let lookup = MapLookup([("payload".to_string(), payload.clone())].into());
    let expr = action_value_to_expr(&json!("{{ payload }}")).unwrap();
    assert_eq!(evaluate(&expr, &lookup).unwrap(), payload);
}

#[test]
fn action_value_to_expr_yaml_scalar_typing() {
    assert_eq!(
        action_value_to_expr(&json!(42)).unwrap(),
        Expr::NumberLiteral(42.0)
    );
    assert_eq!(
        action_value_to_expr(&json!(true)).unwrap(),
        Expr::BoolLiteral(true)
    );
}

#[test]
fn action_value_to_expr_rejects_direct_object() {
    let err = action_value_to_expr(&json!({ "a": 1 })).unwrap_err();
    assert!(
        err.contains("object values are not supported"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("{{"),
        "error should mention whole-value interpolation: {err}"
    );
}

#[test]
fn action_value_to_expr_rejects_direct_array() {
    let err = action_value_to_expr(&json!([1, 2, 3])).unwrap_err();
    assert!(
        err.contains("array values are not supported"),
        "unexpected error: {err}"
    );
    assert!(
        err.contains("{{"),
        "error should mention whole-value interpolation: {err}"
    );
}

// ---------------------------------------------------------------------------
// `proxy.with` authoring surface
// ---------------------------------------------------------------------------

/// Pull the single `Proxy` control action out of a parsed stack.
fn proxy_action(fm: &Value, signal: LifecycleSignal) -> (Expr, ProxyWith) {
    let config = parse_lifecycle_config(fm, dummy_path()).expect("config parses");
    let stack = config.stack(signal).expect("stack present");
    let action = stack[0].actions.last().expect("at least one action");
    match &action.kind {
        LifecycleActionKind::LifecycleControl(LifecycleControlAction::Proxy { target, with }) => {
            (target.clone(), with.clone())
        }
        other => panic!("expected a proxy control action, got: {other:?}"),
    }
}

#[test]
fn proxy_with_omitted_yields_empty_overlay() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"action": "proxy", "target": "@next.md"}}]
        }
    });
    let (target, with) = proxy_action(&fm, LifecycleSignal::Initialize);
    assert_eq!(target, Expr::StringLiteral("@next.md".into()));
    assert!(with.is_empty(), "omitted `with:` installs an empty overlay");
}

#[test]
fn proxy_with_empty_mapping_equals_omission() {
    // `with: {}` parses and is equivalent to omitting `with:` — the spec makes
    // this an explicit equivalence, so compare the whole parsed action rather
    // than only asserting emptiness.
    let omitted = json!({
        "initialize": {
            "stack": [{"action": {"action": "proxy", "target": "@next.md"}}]
        }
    });
    let empty = json!({
        "initialize": {
            "stack": [{"action": {"action": "proxy", "target": "@next.md", "with": {}}}]
        }
    });
    assert_eq!(
        proxy_action(&omitted, LifecycleSignal::Initialize),
        proxy_action(&empty, LifecycleSignal::Initialize),
    );
}

#[test]
fn positional_proxy_yields_empty_overlay() {
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": "@next.md"}}]
        }
    });
    let (_, with) = proxy_action(&fm, LifecycleSignal::Initialize);
    assert!(with.is_empty(), "positional proxy carries no overlay");
}

#[test]
fn proxy_with_types_authored_scalar_and_nested_values() {
    // Every authored JSON shape types through the shared action-value rule,
    // recursed into arrays and objects. Mixed strings stay literals for
    // event-time DM2; a whole-value span keeps its parsed expression, which is
    // what preserves its resolved type at the handoff.
    let fm = json!({
        "failure": {
            "stack": [{"action": {
                "action": "proxy",
                "target": "@next.md",
                "with": {
                    "label": "phase-{{ iteration }}",
                    "attempt": "{{ iteration }}",
                    "ready": true,
                    "count": 3,
                    "cleared": null,
                    "files": ["a.md", "{{ changed }}"],
                    "metadata": {"source": "router", "area": "{{ ctx.area }}"}
                }
            }}]
        }
    });
    let (_, with) = proxy_action(&fm, LifecycleSignal::Failure);
    assert_eq!(with.len(), 7);
    assert_eq!(
        with.get("label"),
        Some(&ProxyWithValue::Scalar(Expr::StringLiteral(
            "phase-{{ iteration }}".into()
        )))
    );
    assert_eq!(
        with.get("attempt"),
        Some(&ProxyWithValue::Scalar(Expr::Variable("iteration".into())))
    );
    assert_eq!(
        with.get("ready"),
        Some(&ProxyWithValue::Scalar(Expr::BoolLiteral(true)))
    );
    assert_eq!(
        with.get("count"),
        Some(&ProxyWithValue::Scalar(Expr::NumberLiteral(3.0)))
    );
    assert_eq!(with.get("cleared"), Some(&ProxyWithValue::Null));
    assert_eq!(
        with.get("files"),
        Some(&ProxyWithValue::Array(vec![
            ProxyWithValue::Scalar(Expr::StringLiteral("a.md".into())),
            ProxyWithValue::Scalar(Expr::Variable("changed".into())),
        ]))
    );
    let Some(ProxyWithValue::Object(metadata)) = with.get("metadata") else {
        panic!("nested mapping types as an object: {:?}", with.get("metadata"));
    };
    assert_eq!(
        metadata.get("source"),
        Some(&ProxyWithValue::Scalar(Expr::StringLiteral("router".into())))
    );
    assert_eq!(
        metadata.get("area"),
        Some(&ProxyWithValue::Scalar(Expr::Variable("ctx.area".into())))
    );
    assert_eq!(with.get("absent"), None);
}

#[test]
fn proxy_with_rejects_unparseable_whole_value_span() {
    // A whole-value span is parsed at authoring time, so a malformed expression
    // is a parse error naming the exact nested path — not an event-time
    // surprise.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {
                "action": "proxy",
                "target": "@next.md",
                "with": {"metadata": {"area": "{{ && }}"}}
            }}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).expect_err("malformed span rejected");
    match &err {
        CompositionError::LifecycleActionInvalidLongForm {
            action, message, ..
        } => {
            assert_eq!(action, "proxy");
            assert!(
                message.contains("action[0].with.metadata.area"),
                "names the exact nested path: {message}"
            );
        }
        other => panic!("expected LifecycleActionInvalidLongForm, got: {other:?}"),
    }
}

#[test]
fn proxy_with_iteration_is_deterministic_and_complete() {
    // Frontmatter parsing normalizes a nested mapping's keys to sorted order
    // before the overlay is built, so iteration is sorted rather than
    // authored. Locked here so a future `preserve_order` change is a visible
    // decision rather than silent drift.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {
                "action": "proxy",
                "target": "@next.md",
                "with": {"zebra": 1, "apple": 2, "mango": 3}
            }}]
        }
    });
    let (_, with) = proxy_action(&fm, LifecycleSignal::Initialize);
    let keys: Vec<&str> = with.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["apple", "mango", "zebra"]);
}

#[test]
fn rejects_non_mapping_proxy_with() {
    // Each non-mapping JSON shape names its own type in the diagnostic.
    for (value, expected) in [
        (json!(7), "number"),
        (json!(true), "boolean"),
        (json!(null), "null"),
        (json!(["a", "b"]), "array"),
        (json!("plain text"), "string"),
        (json!("phase-{{ x }} suffix"), "string"),
    ] {
        let fm = json!({
            "initialize": {
                "stack": [{"action": {"action": "proxy", "target": "@n.md", "with": value}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleProxyWithNotMapping { actual, path, .. } => {
                assert_eq!(actual, expected, "value: {value}");
                assert_eq!(path, "action[0].with");
            }
            other => panic!("expected LifecycleProxyWithNotMapping for {value}, got: {other:?}"),
        }
    }
}

#[test]
fn rejects_whole_mapping_interpolation_proxy_with() {
    // A lone whole-value span reads as "supply the entire mapping from one
    // expression" — a named v1 non-goal with its own diagnostic, distinct from
    // the generic non-mapping error a mixed string produces.
    for raw in ["{{ payload }}", "  {{ payload }}  "] {
        let fm = json!({
            "initialize": {
                "stack": [{"action": {"action": "proxy", "target": "@n.md", "with": raw}}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleProxyWithWholeMapping { raw: got, path, .. } => {
                assert_eq!(got, raw);
                assert_eq!(path, "action[0].with");
            }
            other => panic!("expected LifecycleProxyWithWholeMapping for `{raw}`, got: {other:?}"),
        }
    }
}

#[test]
fn rejects_dynamic_proxy_with_key() {
    // A key carrying a span has no honest dotted representation, so the path
    // stays rooted at `with` and the key travels in its own field.
    for key in ["{{ dynamic }}", "prefix-{{ x }}", "$(echo k)"] {
        let fm = json!({
            "initialize": {
                "stack": [{"action": {
                    "action": "proxy",
                    "target": "@n.md",
                    "with": {key: "value"}
                }}]
            }
        });
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleProxyWithDynamicKey { key: got, path, .. } => {
                assert_eq!(got, key);
                assert_eq!(
                    path, "action[0].with",
                    "an unrepresentable key must not be appended to the dotted path"
                );
            }
            other => panic!("expected LifecycleProxyWithDynamicKey for `{key}`, got: {other:?}"),
        }
    }
}

#[test]
fn static_keys_with_punctuation_are_accepted() {
    // Only interpolation makes a key dynamic. A key that merely fails the
    // safe-path-segment test (a dot) is still a legal static key; it just
    // cannot be appended to a dotted path.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {
                "action": "proxy",
                "target": "@n.md",
                "with": {"dotted.key": 1, "with_underscore": 2, "with-dash": 3}
            }}]
        }
    });
    let (_, with) = proxy_action(&fm, LifecycleSignal::Initialize);
    assert_eq!(with.len(), 3);
    assert_eq!(
        with.get("dotted.key"),
        Some(&ProxyWithValue::Scalar(Expr::NumberLiteral(1.0)))
    );
}

#[test]
fn rejects_with_on_every_other_action() {
    // `with:` is proxy-only. Cover a control verb, a shell action, and a
    // side-effect so the rule is not accidentally scoped to one category.
    for action in [
        json!({"action": "retry", "max_attempts": 2, "with": {"a": 1}}),
        json!({"action": "resume", "message": "again", "with": {"a": 1}}),
        json!({"action": "shell", "command": "ls", "with": {"a": 1}}),
        json!({"action": "message", "message": "hi", "with": {"a": 1}}),
        json!({"action": "set_frontmatter", "file": "s.md", "prop": "p", "value": "v", "with": {"a": 1}}),
    ] {
        let verb = action["action"].as_str().unwrap().to_string();
        let fm = json!({"failure": {"stack": [{"action": action}]}});
        let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
        match err {
            CompositionError::LifecycleProxyOnlyParameter {
                verb: got, param, ..
            } => {
                assert_eq!(got, verb);
                assert_eq!(param, "with");
            }
            other => panic!("expected LifecycleProxyOnlyParameter for `{verb}`, got: {other:?}"),
        }
    }
}

#[test]
fn positional_proxy_plus_sibling_with_stays_ambiguous() {
    // Acceptance criterion 19: the existing ambiguous-action diagnostic is
    // already correct here and its rewrite must point at key/value form.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {"proxy": "prompts/next.md", "with": {"spec": "{{ spec }}"}}}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleStackAmbiguous { message, .. } => {
            assert!(message.contains("action: proxy"), "got: {message}");
            assert!(message.contains("`proxy: ...`"), "got: {message}");
        }
        other => panic!("expected LifecycleStackAmbiguous, got: {other:?}"),
    }
}

#[test]
fn proxy_with_exception_does_not_widen_the_object_parameter_rule() {
    // The exception is exact: `with` on `proxy` only. Another proxy field
    // still hits the generic "direct parameter maps are unsupported" rule.
    let fm = json!({
        "initialize": {
            "stack": [{"action": {
                "action": "proxy",
                "target": {"nested": "map"},
                "with": {"a": 1}
            }}]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleObjectDataThroughInterpolationParameter { param, verb, .. } => {
            assert_eq!(param, "target");
            assert_eq!(verb, "proxy");
        }
        other => panic!("expected object-data rejection for `target`, got: {other:?}"),
    }
}

#[test]
fn proxy_with_diagnostic_names_stack_and_action_index() {
    // The dotted path must locate the exact action: second stack item, second
    // action in its array.
    let fm = json!({
        "failure": {
            "stack": [
                {"action": {"info": "first"}},
                {"action": [
                    {"info": "noise"},
                    {"action": "proxy", "target": "@n.md", "with": "not a mapping"}
                ]}
            ]
        }
    });
    let err = parse_lifecycle_config(&fm, dummy_path()).unwrap_err();
    match err {
        CompositionError::LifecycleProxyWithNotMapping { property, path, .. } => {
            assert_eq!(property, "failure.stack[1]");
            assert_eq!(path, "action[1].with");
        }
        other => panic!("expected LifecycleProxyWithNotMapping, got: {other:?}"),
    }
}

/// Parse `markdown` the way composition does — real YAML frontmatter through
/// Darkmatter — and hand the resulting frontmatter to the lifecycle parser.
///
/// The `json!` fixtures above skip YAML entirely, so they cannot see how an
/// authored scalar is typed or how YAML normalizes a mapping key. This is the
/// shipped read path.
fn lifecycle_from_markdown(markdown: &str) -> Result<LifecycleConfig, CompositionError> {
    let md = darkmatter::markdown::Markdown::try_from_content(markdown.to_string())
        .expect("fixture frontmatter is well-formed YAML");
    let fm = serde_json::Value::Object(
        md.frontmatter()
            .as_map()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    );
    parse_lifecycle_config(&fm, dummy_path())
}

#[test]
fn proxy_with_authored_yaml_preserves_native_and_quoted_scalar_types() {
    // Native YAML scalars keep their types; quoting makes them strings. The
    // overlay is the pre-schema handoff input, so this distinction must
    // survive parse rather than be flattened to text.
    let config = lifecycle_from_markdown(
        "---\n\
         failure:\n\
         \x20   stack:\n\
         \x20       - action:\n\
         \x20             action: proxy\n\
         \x20             target: \"@next.md\"\n\
         \x20             with:\n\
         \x20                 native_bool: true\n\
         \x20                 quoted_bool: \"true\"\n\
         \x20                 native_number: 3\n\
         \x20                 quoted_number: \"3\"\n\
         \x20                 native_null: null\n\
         \x20                 span_bool: \"{{ true }}\"\n\
         \x20                 mixed: \"phase-{{ iteration }}\"\n\
         \x20                 nested:\n\
         \x20                     area: \"{{ ctx.area }}\"\n\
         \x20                 list:\n\
         \x20                     - a\n\
         \x20                     - \"{{ b }}\"\n\
         ---\nbody\n",
    )
    .expect("authored YAML parses");

    let stack = config.stack(LifecycleSignal::Failure).expect("failure stack");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Proxy { with, .. }) =
        &stack[0].actions[0].kind
    else {
        panic!("expected a proxy control action");
    };

    let scalar = |key: &str| match with.get(key) {
        Some(ProxyWithValue::Scalar(expr)) => expr.clone(),
        other => panic!("`{key}` should type as a scalar, got: {other:?}"),
    };

    assert_eq!(scalar("native_bool"), Expr::BoolLiteral(true));
    assert_eq!(scalar("quoted_bool"), Expr::StringLiteral("true".into()));
    assert_eq!(scalar("native_number"), Expr::NumberLiteral(3.0));
    assert_eq!(scalar("quoted_number"), Expr::StringLiteral("3".into()));
    assert_eq!(with.get("native_null"), Some(&ProxyWithValue::Null));
    // The whole-value span parses to the expression, not to its raw text —
    // that is what lets it resolve to a bool rather than the string "true".
    assert_eq!(scalar("span_bool"), Expr::BoolLiteral(true));
    assert_eq!(
        scalar("mixed"),
        Expr::StringLiteral("phase-{{ iteration }}".into())
    );
    let Some(ProxyWithValue::Object(nested)) = with.get("nested") else {
        panic!("`nested` should type as an object: {:?}", with.get("nested"));
    };
    assert_eq!(
        nested.get("area"),
        Some(&ProxyWithValue::Scalar(Expr::Variable("ctx.area".into())))
    );
    assert_eq!(
        with.get("list"),
        Some(&ProxyWithValue::Array(vec![
            ProxyWithValue::Scalar(Expr::StringLiteral("a".into())),
            ProxyWithValue::Scalar(Expr::Variable("b".into())),
        ]))
    );
}

#[test]
fn proxy_with_authored_yaml_rejects_dynamic_key() {
    // The dynamic-key rule must survive real YAML quoting, where a key
    // carrying a span has to be quoted to parse at all.
    let err = lifecycle_from_markdown(
        "---\n\
         initialize:\n\
         \x20   stack:\n\
         \x20       - action:\n\
         \x20             action: proxy\n\
         \x20             target: \"@next.md\"\n\
         \x20             with:\n\
         \x20                 \"{{ key }}\": 1\n\
         ---\nbody\n",
    )
    .unwrap_err();
    assert!(
        matches!(
            err,
            CompositionError::LifecycleProxyWithDynamicKey { ref key, .. } if key == "{{ key }}"
        ),
        "got: {err:?}"
    );
}

#[test]
fn proxy_with_authored_yaml_non_string_key_is_normalized_to_a_static_string() {
    // YAML permits a non-string mapping key (`1:`). Frontmatter parsing
    // normalizes it to the string `"1"` before the lifecycle parser runs, so
    // there is no unrepresentable-key case to diagnose here — it is simply a
    // static key naming a target property called `1`.
    let config = lifecycle_from_markdown(
        "---\n\
         initialize:\n\
         \x20   stack:\n\
         \x20       - action:\n\
         \x20             action: proxy\n\
         \x20             target: \"@next.md\"\n\
         \x20             with:\n\
         \x20                 1: one\n\
         ---\nbody\n",
    )
    .expect("a numeric YAML key normalizes to a static string key");
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Proxy { with, .. }) =
        &stack[0].actions[0].kind
    else {
        panic!("expected a proxy control action");
    };
    assert_eq!(
        with.get("1"),
        Some(&ProxyWithValue::Scalar(Expr::StringLiteral("one".into())))
    );
}

#[test]
fn proxy_with_authored_yaml_empty_mapping_is_accepted() {
    let config = lifecycle_from_markdown(
        "---\n\
         initialize:\n\
         \x20   stack:\n\
         \x20       - action:\n\
         \x20             action: proxy\n\
         \x20             target: \"@next.md\"\n\
         \x20             with: {}\n\
         ---\nbody\n",
    )
    .expect("`with: {}` parses");
    let stack = config
        .stack(LifecycleSignal::Initialize)
        .expect("initialize stack");
    let LifecycleActionKind::LifecycleControl(LifecycleControlAction::Proxy { with, .. }) =
        &stack[0].actions[0].kind
    else {
        panic!("expected a proxy control action");
    };
    assert!(with.is_empty());
}

