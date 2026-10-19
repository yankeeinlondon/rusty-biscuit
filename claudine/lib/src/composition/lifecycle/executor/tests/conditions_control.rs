//! conditions control executor tests.

use super::*;

#[test]
fn when_false_skips_item_when_true_runs() {
    let config = parse_lifecycle_config(
        &json!({
            "success": [
                { "when": "flag == 'yes'", "then": [{ "say": "matched" }] },
                { "when": "flag == 'no'", "then": [{ "say": "never" }] },
            ],
        }),
        Path::new("t.md"),
    )
    .unwrap();

    let fm = map(json!({"flag": "yes"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );

    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Speech("matched".to_string())]);
}

#[test]
fn omitted_when_always_runs() {
    let config = parse_lifecycle_config(
        &json!({ "success": [{ "warn": "always" }] }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(recorder.events(), vec![Emitted::Warn("always".to_string())]);
}

/// A `when:` guard calling an unknown function fails the event closed: the
/// outcome carries an evaluation error and the guarded action dispatches
/// nothing. (A misspelled *property* is an absent property, so its guard is
/// falsy and the item is skipped — see `binding_contract`.)
#[test]
fn when_unknown_function_fails_closed() {
    let config = parse_lifecycle_config(
        &json!({ "success": [{ "when": "spec_fil()", "then": [{ "message": "guarded" }] }] }),
        Path::new("t.md"),
    )
    .unwrap();
    // `spec_file` is a property; the guard calls a function that does not exist.
    let fm = map(json!({"spec_file": "x"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert!(
        outcome.evaluation_error.is_some(),
        "an unknown `when:` function must fail closed through the evaluation channel"
    );
    assert!(
        outcome.action_error.is_none(),
        "a guard raise is an evaluation error, not a dispatch failure"
    );
    assert!(
        recorder.events().is_empty(),
        "no side effect dispatches when the guard fails closed"
    );
}

/// A `when:` guard over an absent property with an `|| false` fallback yields
/// false, so the item is skipped cleanly with no error and no side effect. The
/// fallback chooses a value; it is not needed to make the guard legal.
#[test]
fn when_guarded_fallback_false_skips_cleanly() {
    let config = parse_lifecycle_config(
        &json!({
            "success": [{ "when": "maybe_missing || false", "then": [{ "message": "guarded" }] }],
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert!(recorder.events().is_empty());
}

/// The same fallback form, but the fallback yields true, so the item's action
/// runs.
#[test]
fn when_guarded_fallback_true_runs_action() {
    let config = parse_lifecycle_config(
        &json!({ "success": [{ "when": "maybe_missing || true", "then": [{ "message": "guarded" }] }] }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("guarded".to_string())]
    );
}

/// Regression: a `when:` referencing a known frontmatter key runs the action
/// when it resolves truthy and skips it (no error) when it resolves falsy.
#[test]
fn when_known_key_runs_when_truthy_skips_when_falsy() {
    let config = parse_lifecycle_config(
        &json!({
            "success": [
                { "when": "ready", "then": [{ "message": "ran" }] },
                { "when": "blocked", "then": [{ "message": "never" }] },
            ],
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({"ready": true, "blocked": false}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("ran".to_string())]
    );
}

#[test]
fn array_actions_run_in_order_then_stop_at_control() {
    let config = parse_lifecycle_config(
        &json!({ "failure": [{ "say": "one" }, { "message": "two" }, "stop"] }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome.control, Some(StackControl::Stop));
    assert_eq!(
        recorder.events(),
        vec![
            Emitted::Speech("one".to_string()),
            Emitted::Message("two".to_string()),
        ]
    );
}

#[test]
fn control_action_terminates_remaining_items() {
    let config = parse_lifecycle_config(
        &json!({ "failure": [{ "when": "true", "then": ["stop"] }, { "say": "unreached" }] }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    assert_eq!(outcome.control, Some(StackControl::Stop));
    assert!(recorder.events().is_empty());
}

#[test]
fn err_global_visible_in_failure_stack_when() {
    let config = parse_lifecycle_config(
        &json!({
            "failure": [{ "when": "err.variant == 'Io'", "then": [{ "stderr": "saw io error" }] }],
        }),
        Path::new("t.md"),
    )
    .unwrap();
    let fm = map(json!({}));
    let err = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "disk full".to_string(),
        snapshot: None,
        property: None,
        reason: crate::composition::LifecycleEvaluationReason::Expression,
        cause: None,
    };
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        Some(&err),
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    context.execute_event(&config);
    assert_eq!(
        recorder.events(),
        vec![Emitted::Stderr("saw io error".to_string())]
    );
}



/// Run `signal`'s stack from `frontmatter` against `fm` with a shell that
/// exits `shell_code`, returning the outcome, the emitted events, and the
/// shell commands that ran.
fn run_stack(
    frontmatter: Value,
    signal: LifecycleSignal,
    fm: Value,
    shell_code: i32,
) -> (LifecycleEventOutcome, Vec<Emitted>, Vec<String>) {
    let config = parse_lifecycle_config(&frontmatter, Path::new("t.md")).unwrap();
    let fm = map(fm);
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(shell_code);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        signal,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    (outcome, recorder.events(), shell.commands())
}

fn info(text: &str) -> Emitted {
    Emitted::Info(text.to_string())
}

/// Only the selected body runs: its effects happen in order and its raising
/// operand surfaces at the selected path, never the unselected one.
#[test]
fn only_the_selected_branch_evaluates_its_operands_and_runs_effects() {
    for (flag, expected_info, expected_shell) in
        [(true, "then", "then.sh"), (false, "else", "else.sh")]
    {
        let (outcome, events, commands) = run_stack(
            json!({ "start": [{
                "when": "flag",
                "then": [
                    { "info": "then" },
                    { "shell": "then.sh" },
                    { "info": "{{ no_such_fn() }}" },
                ],
                "else": [
                    { "info": "else" },
                    { "shell": "else.sh" },
                    { "info": "{{ no_such_fn() }}" },
                ],
            }]}),
            LifecycleSignal::Start,
            json!({ "flag": flag }),
            0,
        );
        let selected = if flag { "start[0].then[2].info" } else { "start[0].else[2].info" };
        let error = outcome
            .evaluation_error
            .as_ref()
            .unwrap_or_else(|| panic!("flag={flag}: the selected raise must surface"));
        assert_eq!(error.property.as_deref(), Some(selected), "flag={flag}");
        assert_eq!(events, vec![info(expected_info)], "flag={flag}");
        assert_eq!(commands, vec![expected_shell.to_string()], "flag={flag}");
    }
}

/// The unselected body's raising operand is never evaluated: with the raise
/// only on the unselected side, the event completes cleanly.
#[test]
fn an_unselected_raising_operand_is_never_evaluated() {
    for flag in [true, false] {
        let raising = json!([{ "say": "{{ no_such_fn() }}" }, { "shell": "never.sh" }]);
        let clean = json!([{ "info": "selected" }]);
        let (then_body, else_body) = if flag { (clean, raising) } else { (raising, clean) };
        let (outcome, events, commands) = run_stack(
            json!({ "start": [
                { "when": "flag", "then": then_body, "else": else_body },
                { "info": "after" },
            ]}),
            LifecycleSignal::Start,
            json!({ "flag": flag }),
            0,
        );
        assert_eq!(outcome, LifecycleEventOutcome::default(), "flag={flag}");
        assert_eq!(events, vec![info("selected"), info("after")], "flag={flag}");
        assert!(commands.is_empty(), "flag={flag}: {commands:?}");
    }
}

/// Mixed text is parsed with the stack but never evaluated there: a span that
/// parses and fails only when evaluated is accepted in an unselected branch,
/// and nothing in that branch runs. Selecting it shows the failure is real.
#[test]
fn an_unselected_mixed_span_that_fails_only_at_evaluation_is_accepted_and_skipped() {
    for flag in [true, false] {
        let raising = json!([
            { "say": "status {{ no_such_fn(1) }}" },
            { "info": "deep {{ fm.missing.deep }}" },
            { "set": { "note": "status {{ no_such_fn(2) }}" } },
            { "shell": "never.sh" },
        ]);
        let clean = json!([{ "info": "selected" }]);
        let (then_body, else_body) = if flag { (clean, raising) } else { (raising, clean) };
        let (outcome, events, commands) = run_stack(
            json!({ "start": [{ "when": "flag", "then": then_body, "else": else_body }] }),
            LifecycleSignal::Start,
            json!({ "flag": flag }),
            0,
        );
        assert_eq!(outcome, LifecycleEventOutcome::default(), "flag={flag}");
        assert_eq!(events, vec![info("selected")], "flag={flag}");
        assert!(commands.is_empty(), "flag={flag}: {commands:?}");
    }

    let (outcome, events, _) = run_stack(
        json!({ "start": [{ "when": "true", "then": [{ "say": "status {{ no_such_fn(1) }}" }] }] }),
        LifecycleSignal::Start,
        json!({}),
        0,
    );
    assert!(outcome.evaluation_error.is_some(), "selected, the span fails at evaluation");
    assert!(events.is_empty(), "{events:?}");
}

/// A guard is evaluated exactly once, when its item is reached: a `set` in the
/// selected body that falsifies the guard neither aborts the body nor runs
/// `else`, while the next guard over the same property sees the mutation, as
/// do later actions inside and outside the nested branch.
#[test]
fn a_guard_is_evaluated_once_and_set_is_visible_to_later_actions_and_guards() {
    let (outcome, events, _) = run_stack(
        json!({ "success": [
            {
                "when": "armed",
                "then": [
                    { "set": { "armed": false, "phase": "inner" } },
                    { "info": "armed={{ armed }}" },
                    { "when": "phase == 'inner'", "then": [
                        { "set": { "phase": "deep" } },
                        { "info": "deep={{ phase }}" },
                    ]},
                    { "info": "after-nested={{ phase }}" },
                ],
                "else": [{ "info": "else ran" }],
            },
            { "info": "outside={{ phase }}" },
            { "when": "armed", "then": [{ "info": "still armed" }], "else": [{ "info": "disarmed" }] },
        ]}),
        LifecycleSignal::Success,
        json!({ "armed": true, "phase": "start" }),
        0,
    );
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(
        events,
        vec![
            info("armed=false"),
            info("deep=deep"),
            info("after-nested=deep"),
            info("outside=deep"),
            info("disarmed"),
        ]
    );
}

/// A guard that raises is an evaluation error at its own authored path: it
/// never selects `else`, and nothing after it in any enclosing list runs.
#[test]
fn a_raising_nested_guard_fails_the_event_without_selecting_else() {
    let (outcome, events, commands) = run_stack(
        json!({ "start": [
            { "when": "true", "then": [
                { "info": "before" },
                {
                    "when": "no_such_fn()",
                    "then": [{ "info": "then ran" }],
                    "else": [{ "info": "else ran" }, { "shell": "else.sh" }],
                },
                { "info": "inner sibling" },
            ]},
            { "info": "outer sibling" },
        ]}),
        LifecycleSignal::Start,
        json!({}),
        0,
    );
    let error = outcome
        .evaluation_error
        .as_ref()
        .expect("a raising guard is an evaluation error");
    assert_eq!(error.property.as_deref(), Some("start[0].then[1].when"));
    assert!(outcome.action_error.is_none(), "{outcome:?}");
    assert!(outcome.control.is_none(), "{outcome:?}");
    assert!(outcome.has_evaluation_error());
    assert_eq!(events, vec![info("before")]);
    assert!(commands.is_empty(), "{commands:?}");
}

/// A directive two levels down ends the whole event: no later sibling in the
/// directive's list, its parent's list, or the root list runs. Covered in both
/// a `then` and an `else` body.
#[test]
fn a_depth_two_directive_terminates_every_enclosing_list() {
    for (flag, expected) in [(true, "then-inner"), (false, "else-inner")] {
        let (outcome, events, commands) = run_stack(
            json!({ "failure": [
                { "info": "root-first" },
                { "when": "true", "then": [
                    { "when": "flag",
                      "then": [{ "info": "then-inner" }, "stop"],
                      "else": [{ "info": "else-inner" }, "stop"] },
                    { "info": "depth-one sibling" },
                    { "shell": "unreached.sh" },
                ]},
                { "info": "root sibling" },
            ]}),
            LifecycleSignal::Failure,
            json!({ "flag": flag }),
            0,
        );
        assert_eq!(outcome.control, Some(StackControl::Stop), "flag={flag}");
        assert_eq!(events, vec![info("root-first"), info(expected)], "flag={flag}");
        assert!(commands.is_empty(), "flag={flag}: {commands:?}");
    }
}

/// A nested failing shell under a bundle `no_error` is suppressed and the
/// stack continues past the branch; without `no_error` the same failure
/// stops every enclosing list as an action error.
#[test]
fn nested_dispatch_failure_obeys_no_error_and_otherwise_stops_the_event() {
    for (no_error, expected) in [
        (true, vec![info("in-bundle"), info("after branch")]),
        (false, vec![info("in-bundle")]),
    ] {
        let (outcome, events, commands) = run_stack(
            json!({ "success": [
                { "when": "true", "then": [
                    { "info": "in-bundle", "shell": "fails.sh", "no_error": no_error },
                ]},
                { "info": "after branch" },
            ]}),
            LifecycleSignal::Success,
            json!({}),
            7,
        );
        assert_eq!(commands, vec!["fails.sh".to_string()]);
        assert_eq!(events, expected, "no_error={no_error}");
        assert!(outcome.evaluation_error.is_none(), "{outcome:?}");
        assert_eq!(outcome.action_error.is_some(), !no_error, "{outcome:?}");
    }
}

/// `no_error` never suppresses an expression-evaluation error, also when the
/// raising operand sits in a nested bundle: the stack halts on the evaluation
/// channel and the bundle's later verbs do not run.
#[test]
fn nested_no_error_does_not_suppress_an_evaluation_error() {
    let (outcome, events, commands) = run_stack(
        json!({ "success": [
            { "when": "true", "then": [
                { "say": "{{ no_such_fn() }}", "shell": "later.sh", "no_error": true },
            ]},
            { "info": "after branch" },
        ]}),
        LifecycleSignal::Success,
        json!({}),
        0,
    );
    let error = outcome
        .evaluation_error
        .as_ref()
        .expect("an evaluation raise halts despite no_error");
    assert_eq!(error.property.as_deref(), Some("success[0].then[0].say"));
    assert!(outcome.action_error.is_none());
    assert!(events.is_empty(), "{events:?}");
    assert!(commands.is_empty(), "{commands:?}");
}
