//! retry resume harness-loop tests.

use super::*;

#[test]
fn dispatch_retry_from_failure_continues_and_resets_guard() {
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
    guard.mark_provider_launched();
    // Mark a Failure terminal as already emitted to model the live call site.
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();

    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 2,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
        other => panic!("expected Continue, got {other:?}"),
    }
    // Guard was reset so the retried attempt can emit a fresh terminal.
    assert_eq!(guard.terminal_signal(), None);
}

#[test]
fn dispatch_retry_from_finalize_continues_and_resets_guard() {
    // `finalize` is a last-chance recovery surface: a `finalize.stack`
    // ending in `retry` must re-enter the loop exactly as `failure` does.
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
    guard.mark_provider_launched();
    // Model the live call site: a terminal signal and `finalize` already
    // fired this iteration before the finalize stack's control dispatches.
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    assert!(guard.record_event_emission(LifecycleSignal::Finalize));
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();

    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 1,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Continue { next_attempt } => assert_eq!(next_attempt, 2),
        other => panic!("expected Continue, got {other:?}"),
    }
    // Guard was reset so the retried attempt can emit a fresh terminal.
    assert_eq!(guard.terminal_signal(), None);
}

#[test]
fn dispatch_resume_from_finalize_seeds_prompt_state() {
    // `resume` is valid at `finalize` too (parity with `failure`).
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
    guard.mark_provider_launched();
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();

    let outcome = outcome_with(StackControl::Resume {
        message: "finish the task".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    assert!(matches!(action, TerminalControlAction::Continue { .. }));
    assert_eq!(state.next_prompt_override.as_deref(), Some("finish the task"));
    assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-1"));
}

#[test]
fn dispatch_retry_exhausts_after_budget() {
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
    // Pre-seed the retry budget to ceiling 2 (max_attempts 1 firing at 1).
    let mut budgets = ControlBudgets {
        retry: Some(2),
        resume: None,
    };
    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 1,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    // attempt 2 has reached the ceiling → fall through (no continue).
    let action = dispatch_terminal_control(
        &outcome,
        2,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    assert!(matches!(action, TerminalControlAction::Fallthrough));
}

#[test]
fn dispatch_resume_with_session_seeds_prompt_state() {
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
    guard.mark_provider_launched();
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let outcome = outcome_with(StackControl::Resume {
        message: "please finish the task".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        Some("sess-42"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    assert!(matches!(
        action,
        TerminalControlAction::Continue { next_attempt: 2 }
    ));
    assert_eq!(state.next_resume_session_id.as_deref(), Some("sess-42"));
    assert_eq!(
        state.next_prompt_override.as_deref(),
        Some("please finish the task")
    );
}

#[test]
fn dispatch_resume_without_session_aborts_typed() {
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
    let outcome = outcome_with(StackControl::Resume {
        message: "x".to_string(),
        max_attempts: 1,
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
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Abort(err) => {
            assert!(
                err.to_string().contains("requires a live provider session"),
                "unexpected: {err}"
            );
        }
        other => panic!("expected Abort, got {other:?}"),
    }
}


