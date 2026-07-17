//! terminal routing harness-loop tests.

use super::*;

#[test]
fn dispatch_defer_aborts_not_implemented() {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let outcome = outcome_with(StackControl::Defer {
        delay: "5m".to_string(),
        reason: Some("later".to_string()),
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Abort(err) => {
            let msg = err.to_string();
            assert!(
                msg.contains("defer")
                    && msg.contains("not implemented")
                    && msg.contains("rendezvous"),
                "expected the defer-not-implemented error, got: {err}"
            );
        }
        other => panic!("expected Abort, got {other:?}"),
    }
}

#[test]
fn dispatch_stop_falls_through() {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let action = dispatch_terminal_control(
        &outcome_with(StackControl::Stop),
        1,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    assert!(matches!(action, TerminalControlAction::Fallthrough));
}

#[test]
fn dispatch_error_aborts_without_changing_stop_semantics() {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let action = dispatch_terminal_control(
        &outcome_with(StackControl::Error {
            reason: Some("durable findings remain".to_string()),
        }),
        1,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Abort(error) => {
            assert_eq!(error.to_string(), "durable findings remain");
        }
        other => panic!("expected Error to abort, got {other:?}"),
    }
}

#[test]
fn dispatch_no_control_falls_through() {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let action = dispatch_terminal_control(
        &LifecycleEventOutcome::default(),
        1,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    assert!(matches!(action, TerminalControlAction::Fallthrough));
}

// -- emit_blocked_finalize_with_err (Finding 5) ------------------------

/// Before the provider launches, the helper selects `Blocked` as the
/// terminal signal (matching `emit_blocked_or_failure`'s pre/post-launch
/// rule) and runs both the blocked and finalize stacks, with `err`
/// available to the stack expression engine.
#[test]
fn emit_blocked_finalize_pre_launch_runs_blocked_then_finalize_with_err() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [
                {"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}},
                {"action": {"append_line": ["events.log", "{{ 'blocked-variant=' + err.variant }}"]}},
            ]
        },
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
            ]
        }
    }));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

    emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    // Pre-launch → the terminal signal is Blocked, and finalize fired.
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec![
            "blocked-kind=LifecycleAction",
            "blocked-variant=materialize",
            "finalize-msg=boom",
        ],
        "blocked stack observes err.kind/err.variant; finalize `when: err` is \
         truthy and observes err.msg"
    );
}

/// Once the provider has launched, the helper selects `Failure` as the
/// terminal signal (the post-launch branch of `emit_blocked_or_failure`).
#[test]
fn emit_blocked_finalize_post_launch_selects_failure() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
        }
    }));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

    emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["failure-ran", "finalize-ran"],
    );
}

/// The materialize-failure call site has no real prompt, so it passes a
/// synthetic prompt whose `frontmatter` is `Value::Null`. The stack-context
/// builder must fall back to an empty frontmatter map (rather than panic or
/// skip the stack), so the guard's own blocked/finalize stacks still fire
/// and `err` remains available.
#[test]
fn emit_blocked_finalize_tolerates_null_frontmatter_materialized() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"action": {"append_line": ["events.log", "{{ 'blocked-kind=' + err.kind }}"]}}]
        },
        "finalize": {
            "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
        }
    }));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");
    // The site-1 synthetic prompt: materialization failed, so there is no
    // frontmatter to carry — `Value::Null` exercises the empty-map fallback.
    let synthetic = materialized(serde_json::Value::Null);

    emit_blocked_finalize_with_err(
        &mut guard,
        &synthetic,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["blocked-kind=LifecycleAction", "finalize-ran"],
        "the guard's blocked/finalize stacks fire and observe err even when the \
         materialized prompt's frontmatter is null"
    );
}

// -- emit_failure_finalize_with_err (post-start setup `?` sites) --------

/// The post-start setup sites (launch / pre-spawn attempt) run
/// after `start` and pre-flight have passed, so their terminal signal is
/// always `Failure` — never `Blocked` — and `finalize` must follow with
/// `err` available to both stacks. Here the guard has emitted `start` but
/// the provider has NOT launched, the case the existing
/// `provider_launched()`-driven helper would mis-route to `Blocked`.
#[test]
fn emit_failure_finalize_forces_failure_when_not_launched() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stderr": "failed",
            "stack": [
                {"action": {"append_line": ["events.log", "{{ 'failure-kind=' + err.kind }}"]}},
                {"action": {"append_line": ["events.log", "{{ 'failure-variant=' + err.variant }}"]}},
            ]
        },
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "{{ 'finalize-msg=' + err.msg }}"]}},
            ]
        }
    }));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
    // Reach `start` without launching the provider — exactly the state at
    // the launch / pre-spawn-attempt `?` sites.
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    assert!(!guard.provider_launched());
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("harness_launch", "boom");

    emit_failure_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    // Terminal is Failure (not Blocked) and finalize fired.
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    // Top-level failure communication fired.
    assert_eq!(
        emitter.events(),
        vec![Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string())]
    );
    // Both stacks ran with `err` available.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec![
            "failure-kind=LifecycleAction",
            "failure-variant=harness_launch",
            "finalize-msg=boom",
        ],
        "failure stack observes err.kind/err.variant; finalize `when: err` is \
         truthy and observes err.msg"
    );
}

/// The materialized prompt for an attempt-execution failure carries the
/// real frontmatter, but the helper must equally tolerate a synthetic
/// `Value::Null` frontmatter (empty-map fallback) without skipping the
/// stacks — mirroring the blocked-helper's null tolerance.
#[test]
fn emit_failure_finalize_tolerates_null_frontmatter() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [{"action": {"append_line": ["events.log", "finalize-ran"]}}]
        }
    }));
    let emitter = RecordingEmitter::default();
    let ctx = LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    };
    let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");
    let synthetic = materialized(serde_json::Value::Null);

    emit_failure_finalize_with_err(
        &mut guard,
        &synthetic,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["failure-ran", "finalize-ran"],
    );
}
