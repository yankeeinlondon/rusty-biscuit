//! proxy harness-loop tests.

use super::*;

#[test]
fn dispatch_proxy_swaps_source_and_resets_guard_for_fresh_run() {
    let fx = fixture(serde_json::json!({}));
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody\n").unwrap();
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
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    // Use an absolute target so resolution is unambiguous.
    let outcome = outcome_with(StackControl::Proxy {
        target: target.display().to_string(),
    });
    let action = dispatch_terminal_control(
        &outcome,
        3,
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
    // Proxy re-enters at attempt 1 for a fresh run.
    assert!(matches!(
        action,
        TerminalControlAction::Continue { next_attempt: 1 }
    ));
    assert_eq!(state.source_path, target);
    // The guard was fully reset (initialize will fire again).
    assert!(!guard.initialize_emitted());
    assert_eq!(guard.terminal_signal(), None);
}


