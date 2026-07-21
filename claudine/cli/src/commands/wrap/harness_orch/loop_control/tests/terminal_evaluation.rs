//! terminal evaluation harness-loop tests.

use super::*;

/// Asserts that a surfaced lifecycle evaluation error names the expected
/// event, unwrapping the already-emitted marker used by terminal helpers.
fn assert_lifecycle_eval_error(
    result: Option<CompositionError>,
    event: &str,
) -> CompositionError {
    let err = result.expect("helper must return Some on a lifecycle evaluation raise");
    let inner = match &err {
        CompositionError::LifecycleEvaluationAlreadyEmitted { inner } => inner.as_ref(),
        other => other,
    };
    match inner {
        CompositionError::LifecycleEvaluationError { event: got, .. } => {
            assert_eq!(
                got, event,
                "expected LifecycleEvaluationError for `{event}`, got `{got}`"
            );
            err
        }
        other => panic!("expected LifecycleEvaluationError, got {other:?}"),
    }
}

/// Pre-launch: a `blocked.stack` `when:` raise must surface as a typed
/// evaluation error naming `blocked`, and the helper must still fire the
/// `failure` and `finalize` stacks (with the evaluation error as `err`) by
/// redesignating the already-taken terminal slot. Without the redesignate
/// fix, the failure stack would be silently refused and "failure-ran" would
/// never appear.
#[test]
fn emit_blocked_finalize_pre_launch_blocked_raise_surfaces_failure_and_finalize() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
            ]
        },
        "failure": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}
            ]
        },
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
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
    // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
    // selects `Blocked` as the terminal signal.
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    let typed = assert_lifecycle_eval_error(result, "blocked");
    assert!(
        typed.to_string().contains("evaluation error"),
        "error message surfaces evaluation error: {}",
        typed
    );
    // Redesignation took effect: terminal signal flipped Blocked → Failure.
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    // The key assertion: both failure and finalize stacks ran with the
    // evaluation error as `err` (the redesignate fix lets failure fire).
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    let logged: Vec<&str> = lines.lines().collect();
    assert!(
        logged.contains(&"failure-ran"),
        "failure stack fired with eval error as err: {logged:?}"
    );
    assert!(
        logged.contains(&"finalize-ran"),
        "finalize stack fired with eval error as err: {logged:?}"
    );
}

/// Post-launch: the helper selects `Failure` as the terminal signal. A
/// `failure.stack` `when:` raise surfaces as a typed evaluation error
/// naming `failure`, and the `finalize` stack still fires with the
/// evaluation error as `err`. Failure is already terminal, so no
/// redesignation is needed.
#[test]
fn emit_blocked_finalize_post_launch_failure_raise_surfaces_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
            ]
        },
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
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
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "failure");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    let logged: Vec<&str> = lines.lines().collect();
    assert!(
        logged.contains(&"finalize-ran"),
        "finalize stack fired with eval error as err: {logged:?}"
    );
    // Finalize fired exactly once — the helper did not re-enter failure.
    assert_eq!(
        logged.iter().filter(|l| **l == "finalize-ran").count(),
        1,
        "finalize fired exactly once (no re-entry into failure)"
    );
}

/// A `finalize.stack` raise surfaces as a typed evaluation error naming
/// `finalize`. The helper must not re-enter finalize, and the (already
/// fired) blocked stack must not fire a second time.
#[test]
fn emit_blocked_finalize_finalize_raise_surfaces_without_reentry() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
        },
        "finalize": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
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

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "finalize");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    let logged: Vec<&str> = lines.lines().collect();
    assert_eq!(
        logged.iter().filter(|l| **l == "blocked-ran").count(),
        1,
        "blocked stack fired exactly once (no re-entry)"
    );
}

/// review-4 regression: the pre-start **missing-source** setup-failure
/// branch routes through `emit_blocked_finalize_with_err` (pre-launch →
/// `Blocked`). A `blocked.when` raise must surface a typed evaluation error
/// — proving the branch no longer swallows it in favor of the generic
/// "source file does not exist" fallback. The surfaced event names the
/// terminal event (`blocked`); the redesignate-to-failure path runs the
/// `failure`/`finalize` stacks but the typed error still reports the slot
/// where the raise occurred.
#[test]
fn missing_source_branch_blocked_raise_surfaces_not_swallowed() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
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
    // Pre-launch — the missing-source branch is reached before the provider
    // launches, so do NOT mark it launched; the helper selects `Blocked`.
    let eng = engine(fx._dir.path());
    // The exact err_info the missing-source branch builds.
    let err_info = LifecycleErrorInfo::from_action_failure(
        "missing_source",
        "source file does not exist: prompt.md",
    );

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    // The terminal slot was `blocked`, so the typed error names `blocked`;
    // the evaluation error is surfaced rather than swallowed.
    let typed = assert_lifecycle_eval_error(result, "blocked");
    let rendered = typed.to_string();
    assert!(
        rendered.contains("evaluation error"),
        "error surfaces the evaluation error: {rendered}"
    );
    // The generic missing-source fallback is NOT the surfaced error.
    assert!(
        !rendered.contains("source file does not exist"),
        "the lifecycle raise supersedes the generic fallback: {rendered}"
    );
}

/// review-4 regression: the pre-start **shell-audit** setup-failure branch
/// routes through `emit_blocked_finalize_with_err`. A `finalize.when` raise
/// (with a clean `blocked`) must surface a typed evaluation error naming
/// `finalize` without re-entering finalize — proving the branch no longer
/// swallows it in favor of the generic "shell audit failed" fallback.
#[test]
fn shell_audit_branch_finalize_raise_surfaces_not_swallowed() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
        },
        "finalize": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
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
    // Pre-launch — the shell-audit branch fires before launch.
    let eng = engine(fx._dir.path());
    // The exact err_info the shell-audit branch builds.
    let err_info = LifecycleErrorInfo::from_action_failure(
        "shell_audit",
        "shell audit failed: 1 denied directive(s) in source page",
    );

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    let typed = assert_lifecycle_eval_error(result, "finalize");
    let rendered = typed.to_string();
    assert!(
        rendered.contains("evaluation error"),
        "error surfaces the evaluation error: {rendered}"
    );
    // The generic shell-audit fallback is NOT the surfaced error.
    assert!(
        !rendered.contains("shell audit failed"),
        "the lifecycle raise supersedes the generic fallback: {rendered}"
    );
    // The clean blocked stack fired exactly once and finalize did not
    // re-enter.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert_eq!(
        lines.lines().filter(|l| *l == "blocked-ran").count(),
        1,
        "blocked stack fired exactly once (no re-entry)"
    );
}

/// `emit_failure_finalize_with_err` — a `failure.stack` raise surfaces as
/// a typed evaluation error naming `failure`, and the `finalize` stack
/// still fires with the evaluation error as `err`.
#[test]
fn emit_failure_finalize_failure_raise_surfaces_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
            ]
        },
        "finalize": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "finalize-ran"]}}
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
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("harness_launch", "boom");

    let result = emit_failure_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "failure");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    let logged: Vec<&str> = lines.lines().collect();
    assert!(
        logged.contains(&"finalize-ran"),
        "finalize stack fired with eval error as err: {logged:?}"
    );
}

/// `emit_failure_finalize_with_err` — a `finalize.stack` raise surfaces as
/// a typed evaluation error naming `finalize`. The failure stack (already
/// fired) must not fire a second time.
#[test]
fn emit_failure_finalize_finalize_raise_surfaces_without_reentry() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [
                {"when": "missing_root == true", "action": {"stderr": "never"}}
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
    assert!(guard.record_event_emission(LifecycleSignal::Start));
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("harness_attempt", "boom");

    let result = emit_failure_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "finalize");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    let logged: Vec<&str> = lines.lines().collect();
    assert_eq!(
        logged.iter().filter(|l| **l == "failure-ran").count(),
        1,
        "failure stack fired exactly once (no re-entry)"
    );
}

/// Precedence: when both `failure` and `finalize` raise after a setup
/// error, the surfaced error must name `finalize` — the latest lifecycle
/// crash — not `failure`. Previously the failure raise hid the finalize
/// raise behind it.
#[test]
fn emit_failure_finalize_both_raise_surfaces_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
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

    let result = emit_failure_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "finalize");
    assert!(guard.finalize_emitted(), "finalize must have fired");
}

/// Precedence: a `success.when` raise followed by a `finalize.when` raise
/// must surface the finalize raise — not the original `success` raise.
/// Drives the same path the runtime takes for a terminal evaluation
/// error: `execute_terminal_event` records the raise, then
/// `handle_terminal_evaluation_error` runs `finalize` carrying it.
#[test]
fn success_raise_then_finalize_raise_surfaces_finalize() {
    let fx = fixture(serde_json::json!({
        "success": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
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
    .expect("the terminal evaluation error halts the run");

    let rendered = err.to_string();
    assert!(
        rendered.contains("`finalize`"),
        "the error must name the finalize event, not success; got: {rendered}"
    );
    assert!(
        !rendered.contains("`success`"),
        "the error must NOT name the success event; got: {rendered}"
    );
}

/// Precedence: a setup-phase `initialize`/`start` raise followed by a
/// `failure.when` raise must surface `failure`, and `finalize` must
/// receive the FAILURE evaluation error as `err` (not the original). The
/// `finalize.stack` interpolates `{{ err.event }}` so we can prove it
/// observed the failure raise.
#[test]
fn setup_raise_then_failure_raise_surfaces_failure_and_threads_into_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [{"when": "failure_typo == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stack": [{
                "when": "err",
                "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
            }]
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

    // Model a `start` stack that raised at event time.
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

    let rendered = err.to_string();
    assert!(
        rendered.contains("`failure`"),
        "the error must name the failure event (failure raised); got: {rendered}"
    );
    assert!(
        !rendered.contains("`start`"),
        "the error must NOT name the start event; got: {rendered}"
    );

    // `finalize` ran with the FAILURE evaluation error as `err` — its
    // appended marker interpolates `err.variant`, which the failure raise
    // fills with `when` (the variant of the failure `when:` raise), not
    // the original `missing_root` text.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert!(
        lines.contains("finalize-saw-when"),
        "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
    );
}

/// Precedence: a `blocked.when` raise (terminal) followed by a catch
/// `finalize.when` raise must surface `finalize`. Pre-launch so the
/// helper selects `Blocked`; the redesignation path runs `failure` (no
/// raise authored), then `finalize` raises.
#[test]
fn emit_blocked_finalize_blocked_raise_then_finalize_raise_surfaces_finalize() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "failure": {
            "stack": [{"when": "err", "action": {"append_line": ["events.log", "failure-ran"]}}]
        },
        "finalize": {
            "stack": [{"when": "also_missing == true", "action": {"stderr": "never"}}]
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
    // Pre-launch — do NOT call `mark_provider_launched()` — so the helper
    // selects `Blocked` as the terminal signal and redesignates to Failure.
    let eng = engine(fx._dir.path());
    let err_info = LifecycleErrorInfo::from_action_failure("materialize", "boom");

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert_lifecycle_eval_error(result, "finalize");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    assert!(guard.finalize_emitted(), "finalize must have fired");
    // The failure stack ran (no raise authored) and saw `err`.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert!(
        lines.contains("failure-ran"),
        "failure stack ran with the original blocked evaluation error as err: {lines}"
    );
}

/// Happy-path regression: with no evaluation raises the helper returns
/// `None` and the caller propagates the original setup error unchanged.
#[test]
fn emit_blocked_finalize_returns_none_when_no_evaluation_error() {
    let fx = fixture(serde_json::json!({
        "blocked": {
            "stack": [{"action": {"append_line": ["events.log", "blocked-ran"]}}]
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

    let result = emit_blocked_finalize_with_err(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        &err_info,
        std::time::Instant::now(),
    );

    assert!(result.is_none(), "no evaluation error → returns None");
    assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    assert!(guard.finalize_emitted(), "finalize still fires on the happy path");
}

// -- Broken-path regression tests: explicit error(...) / routes_to_failure
//    catch paths that previously discarded failure/finalize outcomes ------
//
// These exercise the previously-broken catch paths where an explicit
// lifecycle control (`error(...)`), action-error routing (`routes_to_failure`),
// or terminal-control abort still runs failure/finalize but discarded the
// returned outcomes — swallowing any evaluation error raised by those catch
// events.

/// `run_target_initialize` — a target's `initialize.error(...)` whose catch
/// `failure.when:` raises surfaces the FAILURE evaluation error, not the
/// original `error(...)` reason. Proves the previously-discarded failure
/// outcome now threads through the lifecycle catch protocol.
#[test]
fn target_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
    let fx = fixture(serde_json::json!({
        "initialize": {
            "stack": [{"action": {"error": "target refused"}}]
        },
        "failure": {
            "stderr": "fail",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": { "stderr": "final" }
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

    let action = run_target_initialize(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    match action {
        TargetInitializeAction::Abort(report) => {
            let rendered = report.to_string();
            assert!(
                rendered.contains("`failure`"),
                "the surfaced error must name the failure event; got: {rendered}"
            );
            assert!(
                rendered.contains("evaluation error"),
                "the surfaced error must mention evaluation error; got: {rendered}"
            );
            assert!(
                !rendered.contains("target refused"),
                "the original `error(...)` reason must NOT be the surfaced error; got: {rendered}"
            );
        }
        other => panic!("expected Abort, got {other:?}"),
    }
}

/// `run_target_initialize` — a target's `initialize` action error that
/// `routes_to_failure` whose catch `failure.when:` raises surfaces the
/// FAILURE evaluation error, not the generic "lifecycle initialize failed"
/// fallback. Proves the previously-discarded failure outcome now threads
/// through the lifecycle catch protocol for the action-error path.
#[test]
fn target_initialize_routes_to_failure_with_raise_surfaces_failure_evaluation_error() {
    let fx = fixture(serde_json::json!({
        // A `shell: false` action errors and routes_to_failure(Initialize).
        "initialize": {
            "stack": [{"action": {"shell": "false"}}]
        },
        "failure": {
            "stderr": "fail",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": { "stderr": "final" }
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

    let action = run_target_initialize(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );

    match action {
        TargetInitializeAction::Abort(report) => {
            let rendered = report.to_string();
            assert!(
                rendered.contains("`failure`"),
                "the surfaced error must name the failure event; got: {rendered}"
            );
            assert!(
                rendered.contains("evaluation error"),
                "the surfaced error must mention evaluation error; got: {rendered}"
            );
            assert!(
                !rendered.contains("lifecycle initialize failed"),
                "the generic fallback message must NOT be the surfaced error; got: {rendered}"
            );
        }
        other => panic!("expected Abort, got {other:?}"),
    }
}

/// Start `routes_to_failure` catch path (Location G): when `failure.when`
/// raises after a start action error, the surfaced error must name
/// `failure`, and finalize must receive the FAILURE evaluation error as
/// `err` (not the original action error) so a `finalize.stack` can branch
/// on the failure raise. Simulates the inline `run_harness_loop` code
/// path's primitives directly (record_event_emission + run_event_stack +
/// run_lifecycle_event) since the surrounding function is impractical to
/// call from a unit test.
#[test]
fn start_routes_to_failure_with_raise_surfaces_failure_and_threads_into_finalize() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stderr": "fail",
            "stack": [{
                "when": "missing_root == true",
                "action": {"stderr": "never"}
            }]
        },
        "finalize": {
            "stderr": "final",
            "stack": [{
                "when": "err",
                "action": {"append_line": ["events.log", "finalize-saw-{{err.variant}}"]}
            }]
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
    // Mirror run_harness_loop's pre-start state.
    guard.mark_provider_launched();
    let eng = engine(fx._dir.path());

    let action_error = LifecycleErrorInfo::from_action_failure("shell", "boom");
    let result = run_catch_protocol(
        &mut guard,
        LifecycleSignal::Start,
        LifecycleEventOutcome {
            action_error: Some(action_error),
            ..LifecycleEventOutcome::default()
        },
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        None,
        std::time::Instant::now(),
    );
    let failure_error = result
        .evaluation_error
        .as_ref()
        .expect("the failure `when:` raised");
    assert_eq!(result.evaluation_error_signal, Some(LifecycleSignal::Failure));
    let rendered = CompositionError::lifecycle_evaluation(
        "failure",
        &fx.source_path,
        failure_error,
    )
    .to_string();
    assert!(
        rendered.contains("`failure`"),
        "the surfaced error must name the failure event; got: {rendered}"
    );
    assert!(
        !rendered.contains("`start`"),
        "the surfaced error must NOT name the start event; got: {rendered}"
    );

    // finalize ran with the FAILURE evaluation error as `err` — its
    // appended marker interpolates `err.variant`, which the failure raise
    // fills with `when` (the variant of the failure `when:` raise), not
    // the original `shell` action_error variant.
    let lines = std::fs::read_to_string(&fx.log_path).unwrap();
    assert!(
        lines.contains("finalize-saw-when"),
        "finalize must have observed the failure evaluation error (variant=when); got: {lines}"
    );
}

/// Terminal-control abort catch path (Locations H/I/J): when `finalize.when`
/// raises after a terminal-control Abort decision, the surfaced error must
/// name `finalize` (the catch event's raise), not the original abort
/// reason. Simulates the inline `run_harness_loop` Abort arm directly.
#[test]
fn terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
    let fx = fixture(serde_json::json!({
        "finalize": {
            "stderr": "final",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
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
    // The failure/success event already fired cleanly before the Abort.
    guard.mark_provider_launched();
    guard.record_event_emission(LifecycleSignal::Failure);
    let eng = engine(fx._dir.path());

    // Replicate the Location H/I/J fix: run finalize carrying the abort's
    // err_info; if finalize raises, surface the finalize evaluation error.
    let err_info = LifecycleErrorInfo::from_action_failure("agent_failure", "boom");
    let finalize_outcome = run_lifecycle_event(
        &mut guard,
        LifecycleSignal::Finalize,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        Some(&err_info),
        std::time::Instant::now(),
    );

    let surfaced_err: color_eyre::eyre::Report = if let Some(eval_info) =
        finalize_outcome.evaluation_error.as_ref()
    {
        CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
    } else {
        // The original abort reason would surface here on the happy path.
        eyre!("original abort reason")
    };

    let rendered = surfaced_err.to_string();
    assert!(
        rendered.contains("`finalize`"),
        "the surfaced error must name the finalize event; got: {rendered}"
    );
    assert!(
        rendered.contains("evaluation error"),
        "the surfaced error must mention evaluation error; got: {rendered}"
    );
    assert!(
        !rendered.contains("original abort reason"),
        "the original abort reason must NOT be the surfaced error; got: {rendered}"
    );
}

/// Interrupt branch (review-4 Sites B+C): when the run is interrupted and a
/// `failure.when` raises, `handle_terminal_evaluation_error` must surface a
/// `failure`-named evaluation error and run `finalize` exactly once (the
/// helper owns the finalize run; the interrupt branch must not also run a
/// second finalize). Drives the fixed primitives directly since
/// `run_harness_loop` is impractical from a unit test.
#[test]
fn interrupt_failure_when_raise_surfaces_failure_and_runs_finalize_once() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stderr": "fail",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stderr": "final",
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
    // The provider launched before the interrupt, so the Failure slot path
    // is taken (mirrors the interrupt branch's `execute_terminal_event`).
    guard.mark_provider_launched();
    let eng = engine(fx._dir.path());

    let err_info =
        LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
    let failure_outcome = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Failure,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        Some(&err_info),
        std::time::Instant::now(),
    )
    .outcome;

    let surfaced = handle_terminal_evaluation_error(
        &failure_outcome,
        "failure",
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        std::time::Instant::now(),
    );

    let report = surfaced.expect("failure `when:` raise must surface a halting error");
    let rendered = report.to_string();
    assert!(
        rendered.contains("`failure`"),
        "the surfaced error must name the failure event; got: {rendered}"
    );
    assert!(
        rendered.contains("evaluation error"),
        "the surfaced error must mention evaluation error; got: {rendered}"
    );
    // `handle_terminal_evaluation_error` runs `finalize` once internally; the
    // interrupt branch must NOT run it again (no recursive re-entry).
    assert_eq!(
        line_count(&fx.log_path),
        1,
        "finalize ran exactly once (handler-owned, no double finalize)"
    );
}

/// Interrupt branch (review-4 Sites B+C): a clean `failure` followed by a
/// raising `finalize.when`. `handle_terminal_evaluation_error` returns
/// `None` (failure did not raise), then the interrupt branch's own finalize
/// run raises → a `finalize`-named evaluation error halts the run, and the
/// `Ok((exit_code, ...))` happy path is NOT taken.
#[test]
fn interrupt_finalize_when_raise_surfaces_finalize_evaluation_error() {
    let fx = fixture(serde_json::json!({
        "failure": { "stderr": "fail" },
        "finalize": {
            "stderr": "final",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
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

    let err_info =
        LifecycleErrorInfo::from_action_failure("interrupted", "user interrupted the run");
    let failure_outcome = execute_terminal_event(
        &mut guard,
        LifecycleSignal::Failure,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        Some(&err_info),
        std::time::Instant::now(),
    )
    .outcome;

    // The clean `failure` stack does not raise.
    assert!(
        handle_terminal_evaluation_error(
            &failure_outcome,
            "failure",
            &mut guard,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            std::time::Instant::now(),
        )
        .is_none(),
        "a clean failure must not surface an evaluation error"
    );

    // The interrupt branch then runs `finalize`, which raises here.
    let finalize_outcome = run_lifecycle_event(
        &mut guard,
        LifecycleSignal::Finalize,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        Some(&err_info),
        std::time::Instant::now(),
    );

    let surfaced: Option<color_eyre::eyre::Report> =
        finalize_outcome.evaluation_error.as_ref().map(|eval_info| {
            CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
        });
    let report = surfaced.expect("finalize `when:` raise must halt instead of returning Ok");
    let rendered = report.to_string();
    assert!(
        rendered.contains("`finalize`"),
        "the surfaced error must name the finalize event; got: {rendered}"
    );
    assert!(
        rendered.contains("evaluation error"),
        "the surfaced error must mention evaluation error; got: {rendered}"
    );
}

/// Start control-abort site (review-4 Site A): when the `start`
/// control-dispatch aborts and `finalize.when` raises, the surfaced error
/// must name `finalize` (the catch event's raise), not the original abort
/// reason. The start-abort finalize runs with `None` `err` (no error info is
/// available at that point), so this mirrors
/// `terminal_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error`
/// but with a `None` finalize `err`.
#[test]
fn start_control_abort_with_finalize_raise_surfaces_finalize_evaluation_error() {
    let fx = fixture(serde_json::json!({
        "finalize": {
            "stderr": "final",
            "stack": [{"when": "missing_root == true", "action": {"stderr": "never"}}]
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
    // A terminal slot was taken before the control-abort decision, so the
    // subsequent `finalize` is eligible to fire (its run is gated on a
    // recorded terminal emission).
    guard.mark_provider_launched();
    guard.record_event_emission(LifecycleSignal::Failure);
    let eng = engine(fx._dir.path());

    // Replicate the Site A fix: finalize runs with `None` err; if it raises,
    // surface the finalize evaluation error in place of the abort reason.
    let finalize_outcome = run_lifecycle_event(
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

    let surfaced_err: color_eyre::eyre::Report =
        if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
            CompositionError::lifecycle_evaluation("finalize", &fx.source_path, eval_info).into()
        } else {
            eyre!("original abort reason")
        };

    let rendered = surfaced_err.to_string();
    assert!(
        rendered.contains("`finalize`"),
        "the surfaced error must name the finalize event; got: {rendered}"
    );
    assert!(
        rendered.contains("evaluation error"),
        "the surfaced error must mention evaluation error; got: {rendered}"
    );
    assert!(
        !rendered.contains("original abort reason"),
        "the original abort reason must NOT be the surfaced error; got: {rendered}"
    );
}
