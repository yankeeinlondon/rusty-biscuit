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
        overlay: indexmap::IndexMap::new(),
        location: claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
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

/// A terminal-stack `proxy` to a nonexistent document must be rejected **here**,
/// before the target is adopted — the same way, at the same point, as the
/// identical proxy on the `initialize` route.
///
/// Regression: this path called `claudine::harness::resolve_harness_path`
/// directly, which only joins the reference against the source and never checks
/// that the result exists. A `proxy('missing.md')` therefore resolved to a
/// nonexistent path, was adopted as the active document, and failed later and
/// differently than the same proxy at `initialize`. Both routes now go through
/// the one owner, `claudine::composition::resolve_proxy_target`.
#[test]
fn dispatch_proxy_to_missing_target_aborts_without_adopting_it() {
    let fx = fixture(serde_json::json!({}));
    let missing = fx._dir.path().join("missing.md");
    assert!(!missing.exists(), "the fixture target must not exist");
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
    let mut tracking = ProxyTracking::default();
    let outcome = outcome_with(StackControl::Proxy {
        target: missing.display().to_string(),
        overlay: indexmap::IndexMap::new(),
        location: claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
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
        &mut tracking,
        &fx.term,
        false,
    );

    let TerminalControlAction::Abort(err) = action else {
        panic!("a proxy to a missing document must abort, got {action:?}");
    };
    let rendered = format!("{err:#}");
    assert!(
        rendered.contains("proxy target does not exist"),
        "the abort must carry the shared resolver's existence diagnostic, got: {rendered}"
    );

    // Downstream state must be untouched: a rejected hand-off never
    // half-activates the target.
    assert_eq!(
        state.source_path, fx.source_path,
        "the source document stays active when the hand-off is rejected"
    );
    assert!(
        !tracking.pending,
        "a rejected hand-off must not flag a pending proxy adoption"
    );
    assert!(
        !tracking.chain.contains(&missing),
        "a missing target must never enter the proxy chain"
    );
    assert!(
        guard.initialize_emitted() || guard.terminal_signal().is_some(),
        "the guard must not be reset for a hand-off that was rejected"
    );
}


