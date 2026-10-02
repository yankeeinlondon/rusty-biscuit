//! conditions control executor tests.

use super::*;

#[test]
fn when_false_skips_item_when_true_runs() {
    let config = parse_lifecycle_config(
        &json!({
            "success": {
                "stack": [
                    {"when": "flag == 'yes'", "action": {"say": "matched"}},
                    {"when": "flag == 'no'", "action": {"say": "never"}}
                ]
            }
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
        &json!({"success": {"stack": [{"action": {"warn": "always"}}]}}),
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
        &json!({
            "success": {
                "stack": [{"when": "spec_fil()", "action": {"message": "guarded"}}]
            }
        }),
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
            "success": {
                "stack": [{"when": "maybe_missing || false", "action": {"message": "guarded"}}]
            }
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
        &json!({
            "success": {
                "stack": [{"when": "maybe_missing || true", "action": {"message": "guarded"}}]
            }
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
            "success": {
                "stack": [
                    {"when": "ready", "action": {"message": "ran"}},
                    {"when": "blocked", "action": {"message": "never"}}
                ]
            }
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
        &json!({
            "failure": {
                "stack": [{
                    "action": [{"say": "one"}, {"message": "two"}, "stop"]
                }]
            }
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
        &json!({
            "failure": {
                "stack": [
                    {"action": "stop"},
                    {"action": {"say": "unreached"}}
                ]
            }
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
            "failure": {
                "stack": [{
                    "when": "err.variant == 'Io'",
                    "action": {"stderr": "saw io error"}
                }]
            }
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


