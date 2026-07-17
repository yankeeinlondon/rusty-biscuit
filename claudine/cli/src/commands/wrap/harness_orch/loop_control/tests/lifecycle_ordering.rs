//! lifecycle ordering harness-loop tests.

use super::*;

#[test]
fn success_stack_side_effects_run_exactly_once() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stderr": "succeeded",
            "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
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

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(outcome.control.is_none());
    // The stack's side effect fired exactly once (was twice before the fix).
    assert_eq!(line_count(&fx.log_path), 1, "stack ran exactly once");
    // Top-level success communication fired (the stack stayed success).
    assert_eq!(
        emitter.events(),
        vec![Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string())]
    );
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
}

/// Phase 2: a terminal-phase `success.when` evaluation error is now filed in
/// the dedicated evaluation-error channel, not the tolerated `action_error`.
///
/// The first stack item's `when:` references an undefined frontmatter root,
/// so it *raises* at event time (it does not evaluate cleanly to `false`).
/// The spec (Decision #1) requires distinguishing such an **evaluation**
/// error from a side-effect **dispatch** failure: the former must halt the
/// run, the latter keeps today's log-and-continue policy.
///
/// This asserts only the executor-level classification (Phase 2): the raise
/// surfaces through `evaluation_error` and never lands in `action_error`. The
/// orchestration that turns this into a `finalize`-with-`err` + non-zero
/// outcome is wired in Phase 3.
#[test]
fn success_when_evaluation_error_is_not_swallowed_as_action_error() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "ready"}}
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
    guard.mark_provider_launched();
    let eng = engine(fx._dir.path());

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    // The `when:` raised, so the stack never reached an action: nothing was
    // emitted and no control action fired.
    assert!(outcome.control.is_none(), "no control action ran");
    assert!(emitter.events().is_empty(), "the guarded action never ran");
    // The raise is filed in the dedicated evaluation-error channel, not the
    // tolerated dispatch-failure `action_error` channel the success path
    // would otherwise drop.
    assert!(
        outcome.action_error.is_none(),
        "a terminal-phase `when:` evaluation error must not be filed as an `action_error`"
    );
    assert!(
        outcome.evaluation_error.is_some(),
        "the `when:` raise surfaces through the halting evaluation-error channel"
    );
}

/// Phase 3: a terminal-phase `success` evaluation error halts the run.
///
/// `handle_terminal_evaluation_error` runs `finalize` exactly once with the
/// evaluation error exposed as `err` (so a `when: "err"` finalize branch
/// fires) and returns the typed `LifecycleEvaluationError`. It does **not**
/// fire `failure` (Decision #3): the provider already succeeded.
#[test]
fn success_evaluation_error_runs_finalize_with_err_and_returns_failure() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "ready"}}]
        },
        "finalize": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalized-with-err"]}}]
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

    let success = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );
    assert!(
        success.outcome.evaluation_error.is_some(),
        "the success `when:` raised"
    );

    let err = handle_terminal_evaluation_error(
        &success.outcome,
        "success",
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        std::time::Instant::now(),
    )
    .expect("a terminal evaluation error produces a run failure");

    let rendered = err.to_string();
    assert!(
        rendered.contains("`success`") && rendered.contains("evaluation error"),
        "the run failure names the event and is a typed evaluation error: {rendered}"
    );
    // `finalize` fired exactly once and saw `err` populated (its `when: err`
    // branch ran), proving the error was threaded into the finalize context.
    assert!(guard.finalize_emitted(), "finalize fired");
    assert_eq!(
        line_count(&fx.log_path),
        1,
        "finalize ran once with `err` available"
    );
    // The provider succeeded, so `failure` was NOT fired (Decision #3).
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
}

/// A `success` stack that downgrades to `failure` via explicit `error()`,
/// where the resulting `failure` stack then raises an evaluation error,
/// must surface the error attributed to `failure` — not `success`. After a
/// downgrade, `outcome` holds the failure event's result, so
/// `effective_event` must be `"failure"`; without it, the success caller
/// would hardcode `"success"` and the diagnostic would point at the wrong
/// event.
#[test]
fn downgraded_success_failure_raise_reports_failure_event() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [{"action": {"error": "downgraded"}}]
        },
        "failure": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
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

    let success = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    // The success stack downgraded via `error()`, and the failure stack
    // raised an evaluation error in its `when:`.
    assert!(
        success.outcome.evaluation_error.is_some(),
        "the downgraded failure stack raised"
    );
    assert_eq!(
        success.effective_event, "failure",
        "effective_event must be `failure` after a downgrade"
    );

    let err = handle_terminal_evaluation_error(
        &success.outcome,
        success.effective_event,
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        std::time::Instant::now(),
    )
    .expect("the failure evaluation error halts the run");

    let rendered = err.to_string();
    assert!(
        rendered.contains("`failure`"),
        "the error must name the failure event, not success; got: {rendered}"
    );
    assert!(
        !rendered.contains("`success`"),
        "the error must NOT name the success event; got: {rendered}"
    );
}

/// Regression guard: a `success` stack whose own `when:` raises (no
/// downgrade) keeps `effective_event == "success"`. Confirms the fix did
/// not break the non-downgrading path.
#[test]
fn success_evaluation_error_non_downgrading_reports_success_event() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "unreachable"}}]
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

    let success = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(
        success.outcome.evaluation_error.is_some(),
        "the success `when:` raised"
    );
    assert_eq!(
        success.effective_event, "success",
        "effective_event stays `success` when no downgrade occurred"
    );
}

/// Phase 3: a terminal-phase side-effect **dispatch** failure is NOT
/// escalated — `handle_terminal_evaluation_error` returns `None` and runs no
/// `finalize`, so the caller keeps today's log-and-continue policy.
#[test]
fn terminal_dispatch_failure_keeps_previous_outcome() {
    let fx = fixture(serde_json::json!({
        "finalize": {
            "stack": [{"action": {"append_line": ["events.log", "finalized"]}}]
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
    assert!(guard.record_event_emission(LifecycleSignal::Success));
    let eng = engine(fx._dir.path());

    // A dispatch failure populates `action_error`, never `evaluation_error`.
    let outcome = LifecycleEventOutcome {
        action_error: Some(LifecycleErrorInfo::from_action_failure("shell", "boom")),
        ..Default::default()
    };
    let halted = handle_terminal_evaluation_error(
        &outcome,
        "success",
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        std::time::Instant::now(),
    );
    assert!(halted.is_none(), "a dispatch failure does not halt the run");
    assert!(!guard.finalize_emitted(), "no finalize was forced");
    assert_eq!(line_count(&fx.log_path), 0, "the finalize stack did not run");
}

/// Behavior-matrix counterpart to the `success.when` raise: a terminal-phase
/// `when:` that evaluates **cleanly to `false`** just skips its item — no
/// evaluation error, no halt. This is the crashed-vs-clean-false distinction
/// at the orchestration layer: a clean `false` guard must never be confused
/// with a swallowed raise.
#[test]
fn terminal_clean_false_guard_skips_without_halting() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [{"when": "1 == 2", "action": {"append_line": ["events.log", "ran"]}}]
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

    let success = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    // A clean `false` guard raises nothing and dispatches nothing.
    assert!(
        success.outcome.evaluation_error.is_none(),
        "a clean `false` guard must not file an evaluation error"
    );
    assert!(success.outcome.action_error.is_none());
    assert_eq!(line_count(&fx.log_path), 0, "the guarded action was skipped");

    // The clean false therefore does not halt the run.
    let halted = handle_terminal_evaluation_error(
        &success.outcome,
        "success",
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        std::time::Instant::now(),
    );
    assert!(halted.is_none(), "a clean false guard does not halt the run");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
}

/// Phase 3: a setup-phase evaluation error routes through `failure` then
/// `finalize` (Decision #5), threading the error as `err`, and returns the
/// typed run failure.
#[test]
fn setup_evaluation_error_routes_through_failure_and_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}]
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

    // Model a `start` stack that raised at event time (no terminal recorded).
    let outcome = LifecycleEventOutcome {
        evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
            "when",
            "`when:` references undefined variable `missing_root`",
        )),
        ..Default::default()
    };
    let early = crate::output::error_walker::emit_lifecycle_evaluation_error_early(
        &fx.source_path,
        "start",
        outcome.evaluation_error.as_ref().unwrap(),
        &fx.term,
    );
    let result = run_catch_protocol(
        &mut guard,
        LifecycleSignal::Start,
        outcome,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );
    let err = surface_protocol_evaluation(
        &result,
        LifecycleSignal::Start,
        &fx.source_path,
        Some(early),
        &fx.term,
    );

    assert!(err.to_string().contains("`start`"), "names the start event");
    // Both `failure` and `finalize` ran, each seeing `err` populated.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["failure-ran", "finalize-ran"],
        "setup evaluation error routes through failure then finalize, both with `err`"
    );
    assert!(guard.finalize_emitted(), "finalize fired");
}

/// Phase 3: an evaluation error raised *inside* `finalize` aborts the run
/// without re-entering `finalize` (the re-entry guard). `finalize` fires
/// exactly once and the recovery path returns `Abort` with the typed error.
#[test]
fn finalize_evaluation_error_aborts_without_reentry() {
    let fx = fixture(serde_json::json!({
        "finalize": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "x"}}]
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
    // Model the live call site: a terminal already fired this iteration.
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let eng = engine(fx._dir.path());
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();

    let action = run_finalize_with_recovery(
        &mut guard,
        &fx.materialized,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
        1,
        &mut budgets,
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Claude,
        &mut state,
        &ledger(&fx.source_path),
        false,
    );

    match action {
        TerminalControlAction::Abort(err) => {
            assert!(
                err.to_string().contains("`finalize`"),
                "the abort names the finalize event: {err}"
            );
        }
        other => panic!("expected Abort from a finalize evaluation error, got {other:?}"),
    }
    // `finalize` ran exactly once — the abort did not loop back into it.
    assert!(guard.finalize_emitted(), "finalize fired exactly once");
}

#[test]
fn success_stack_error_routes_to_failure_keeps_success_comm_before_failure() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stderr": "succeeded",
            "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
        },
        "failure": {
            "stderr": "failed",
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
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

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    // Outcome reflects the failure event's run (no Error control surviving).
    assert!(outcome.control.is_none());
    // Success stack ran once (append + error), failure stack ran once.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["ran", "failure-ran"],
        "success stack and failure stack each ran exactly once"
    );
    // The success top-level comm fired FIRST (top-level-before-stack), then
    // the downgrade fired the failure top-level comm. The success comm is
    // NOT suppressed — the spec requires top-level to fire before stack
    // processing, so a later `error()` cannot un-fire it.
    assert_eq!(
        emitter.events(),
        vec![
            Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string()),
            Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
        ]
    );
    // Guard recorded the downgraded terminal signal as Failure.
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
}

#[test]
fn blocked_stack_side_effects_run_exactly_once() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stderr": "blocked",
            "stack": [{"action": {"append_line": ["events.log", "ran"]}}]
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

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Blocked,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(outcome.control.is_none());
    assert_eq!(line_count(&fx.log_path), 1, "blocked stack ran exactly once");
    assert_eq!(
        emitter.events(),
        vec![Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string())]
    );
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
}

/// Top-level communication for `success` fires before any `stack:`
/// communication in the same event.
#[test]
fn success_top_level_communication_fires_before_stack_communication() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stderr": "success-top",
            "stack": [{"action": {"stderr": "success-stack"}}]
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

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Success,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(outcome.control.is_none());
    assert_eq!(
        emitter.events(),
        vec![
            Emitted::Stderr(LifecycleSignal::Success, "success-top".to_string()),
            Emitted::Stderr(LifecycleSignal::Success, "success-stack".to_string()),
        ],
        "top-level communication must fire before stack communication"
    );
}

/// Top-level communication for `blocked` fires before any `stack:`
/// communication in the same event.
#[test]
fn blocked_top_level_communication_fires_before_stack_communication() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stderr": "blocked-top",
            "stack": [{"action": {"stderr": "blocked-stack"}}]
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

    let TerminalEventOutcome { outcome, .. } = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Blocked,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(outcome.control.is_none());
    assert_eq!(
        emitter.events(),
        vec![
            Emitted::Stderr(LifecycleSignal::Blocked, "blocked-top".to_string()),
            Emitted::Stderr(LifecycleSignal::Blocked, "blocked-stack".to_string()),
        ],
        "top-level communication must fire before stack communication"
    );
}

/// Reproduces the exact guard call sequence of the `run_harness_loop`
/// `routes_to_failure(Start)` branch: a `start` stack action errored, so
/// the failure path records `Failure`, runs the error-carrying failure
/// stack, and then must reach `finalize`. Asserts the failure AND finalize
/// stacks each ran exactly once and `finalize_emitted()` is true — proving
/// `finalize` is not skipped (the Finding 2 defect).
#[test]
fn start_stack_action_error_records_failure_then_runs_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stderr": "failed",
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stderr": "finalized",
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
    // The provider never launched (a setup-phase `start` error routes here),
    // but `start` was emitted — mirror the loop's pre-launch state.
    guard.mark_provider_launched();
    let eng = engine(fx._dir.path());

    // --- The fixed `routes_to_failure(Start)` branch sequence ---
    let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
    // 1. Record `Failure` FIRST (the fix). This sets `terminal_emitted`.
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    // 2. Run the error-carrying failure stack via `run_event_stack`.
    let failure_ctx = build_lifecycle_stack_context_for_materialized(
        LifecycleSignal::Failure,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        None,
        None,
        &fx.term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        &eng,
        Some(&action_error),
        None,
        None,
    );
    guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
    // 3. Finalize must now fire (records + runs because terminal_emitted).
    run_lifecycle_event(
        &mut guard,
        LifecycleSignal::Finalize,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        Some(&action_error),
        std::time::Instant::now(),
    );

    // Finalize was NOT skipped.
    assert!(
        guard.finalize_emitted(),
        "finalize must fire after a setup-phase failure"
    );
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    // Both the failure stack and the finalize stack ran exactly once.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["failure-ran", "finalize-ran"],
        "failure stack and finalize stack each ran exactly once"
    );
    // Both top-level comms fired, failure before finalize.
    assert_eq!(
        emitter.events(),
        vec![
            Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
            Emitted::Stderr(LifecycleSignal::Finalize, "finalized".to_string()),
        ]
    );
}

/// Locks in WHY the fix is needed: calling `run_event_stack(Failure, ...)`
/// WITHOUT first `record_event_emission(Failure)` leaves `terminal_emitted`
/// false, so a subsequent `Finalize` is a no-op (the finalize stack never
/// runs). This documents the Finding 2 defect the fix removes.
#[test]
fn failure_stack_without_record_skips_finalize() {
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

    // Defective sequence: run the failure stack directly, no record.
    let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
    let failure_ctx = build_lifecycle_stack_context_for_materialized(
        LifecycleSignal::Failure,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        None,
        None,
        &fx.term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        &eng,
        Some(&action_error),
        None,
        None,
    );
    guard.run_event_stack(LifecycleSignal::Failure, &failure_ctx);
    // `Finalize` is a no-op because no terminal signal was recorded.
    run_lifecycle_event(
        &mut guard,
        LifecycleSignal::Finalize,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    assert!(
        !guard.finalize_emitted(),
        "without record_event_emission(Failure) the finalize is skipped"
    );
    // The failure stack ran but the finalize stack did not.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["failure-ran"],
        "finalize stack must not run when the terminal signal was never recorded"
    );
}

#[test]
fn blocked_stack_error_routes_to_failure_keeps_blocked_comm_before_failure() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stderr": "blocked",
            "stack": [{"action": [{"append_line": ["events.log", "ran"]}, {"error": "downgraded"}]}]
        },
        "failure": {
            "stderr": "failed",
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
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

    execute_terminal_event(
        &mut guard,
        LifecycleSignal::Blocked,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["ran", "failure-ran"],
        "blocked stack and failure stack each ran exactly once"
    );
    // The blocked top-level comm fired FIRST (top-level-before-stack), then
    // the downgrade fired the failure top-level comm.
    assert_eq!(
        emitter.events(),
        vec![
            Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string()),
            Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string()),
        ]
    );
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
}

// -- dispatch_terminal_control runtime-wiring tests --------------------

// -- handoff-failure routing ------------------------------------------------
//
// Phase 12 of `features/2026-07-13-proxy-with`, task 4. A refused hand-off
// leaves the *source* active — `adopt` rejects before it touches document
// identity — so the source still owes its closure. Every `adopt` call site used
// to bare-`?` the failure, which had a visible consequence on the terminal
// routes: `failure` had already fired, so the guard's `Drop` net stayed silent
// (it only fires when `start_emitted && !terminal_emitted`) and the run exited
// **without its owed `finalize`**. A `finalize.stack` that releases a lock or
// posts a status simply never ran, on the path where it mattered most.
//
// Event-awareness is not branched on here — it falls out of the guard's
// emission ledger, which is why these tests drive the real guard rather than a
// stub of it.

/// The regression. `failure` has fired; a hand-off then fails. The owed
/// `finalize` must still run, and must see the hand-off's `err`.
#[test]
fn a_handoff_failure_after_the_terminal_event_still_runs_the_owed_finalize() {
    let fx = fixture(serde_json::json!({
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "finalize-saw-err"]}},
                {"action": {"append_line": ["events.log", "finalize-ran"]}}
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

    // The source ran and failed: `start` and the terminal `failure` are spent.
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));

    let report = route_handoff_failure(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        unresolvable_commit_error(&fx.source_path),
        std::time::Instant::now(),
    );

    assert!(
        guard.finalize_emitted(),
        "the owed `finalize` must fire: the source is still active after a refused \
         hand-off, and the terminal event having already fired is exactly why the \
         Drop net cannot cover this"
    );
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["finalize-saw-err", "finalize-ran"],
        "`finalize` runs once and observes the hand-off failure as `err`"
    );
    assert_eq!(
        emitter
            .events()
            .into_iter()
            .filter(|event| matches!(event, Emitted::Stderr(LifecycleSignal::Failure, _)))
            .count(),
        0,
        "the terminal slot was already taken, so no second `failure` is emitted — \
         no-duplicate-emission comes from the ledger, not from a branch"
    );
    assert!(
        report
            .downcast_ref::<claudine::composition::ProxyCommitError>()
            .is_some(),
        "the hand-off's typed error propagates, not a stringified stand-in"
    );
}

/// Before any terminal event the same failure is a pre-launch `blocked`, and
/// both `blocked` and `finalize` fire. Same helper, opposite ledger state — the
/// routing reads the guard rather than being told.
#[test]
fn a_handoff_failure_before_the_terminal_event_routes_blocked_then_finalize() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "blocked-ran"]}}]
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

    // A `start`-stack proxy: started, never launched a provider.
    assert!(guard.record_event_emission(LifecycleSignal::Start));

    let _ = route_handoff_failure(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        unresolvable_commit_error(&fx.source_path),
        std::time::Instant::now(),
    );

    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().collect::<Vec<_>>(),
        vec!["blocked-ran", "finalize-ran"],
        "no provider launched, so the failure is `blocked` — and `finalize` follows"
    );
    assert_eq!(
        guard.terminal_signal(),
        Some(LifecycleSignal::Blocked),
        "the terminal slot records `blocked`, matching every other pre-launch failure"
    );
}

/// Once `finalize` has fired the failure surfaces directly: no lifecycle event
/// fires at all. This is the third arm of the plan's event-aware rule, and the
/// one that makes "a failure inside `finalize` never re-enters `finalize`"
/// true for hand-offs too.
#[test]
fn a_handoff_failure_after_finalize_surfaces_without_re_emitting() {
    let fx = fixture(serde_json::json!({
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

    assert!(guard.record_event_emission(LifecycleSignal::Start));
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    assert!(guard.record_event_emission(LifecycleSignal::Finalize));

    let report = route_handoff_failure(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        unresolvable_commit_error(&fx.source_path),
        std::time::Instant::now(),
    );

    assert!(
        std::fs::read_to_string(&fx.log_path).is_err_and(|e| e.kind()
            == std::io::ErrorKind::NotFound),
        "the run's closure is already spent: nothing may fire, so the stack never \
         writes its line"
    );
    assert!(
        report
            .downcast_ref::<claudine::composition::ProxyCommitError>()
            .is_some(),
        "the failure still surfaces — silently swallowing it would be the other \
         half of the bug"
    );
}
