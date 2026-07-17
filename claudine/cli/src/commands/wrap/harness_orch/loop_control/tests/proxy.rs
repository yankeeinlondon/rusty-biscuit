//! What a provider attempt may and may not do with a `proxy` control.
//!
//! The dispatch's whole job here is to *ask*. Everything about committing the
//! hand-off — resolving the target, deciding the hop, changing the active
//! document — is asserted in [`super::coordinator_adoption`], because that is
//! where it now lives.

use super::*;

/// The regression this phase closes: the dispatch used to resolve the target,
/// push the chain, mutate `source_path`/`original_ref`, and reset the guard,
/// all from inside a provider attempt. It must now return a request and touch
/// nothing.
#[test]
fn dispatch_proxy_requests_a_handoff_without_touching_active_document() {
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
    let mut overlay = indexmap::IndexMap::new();
    overlay.insert("phase".to_string(), serde_json::json!(2));
    let outcome = outcome_with(StackControl::Proxy {
        target: target.display().to_string(),
        overlay,
        location: claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 1),
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
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );

    let TerminalControlAction::Proxy(request) = action else {
        panic!("a proxy control must produce a request, got {action:?}");
    };
    assert_eq!(request.target(), target.display().to_string());
    assert_eq!(
        request.overlay().get("phase"),
        Some(&serde_json::json!(2)),
        "the evaluated overlay rides on the request"
    );
    assert_eq!(
        request.provenance().source_path(),
        fx.source_path,
        "provenance attributes the hop to the document that authored it"
    );
    assert_eq!(
        request.provenance().location().to_string(),
        "failure.stack[0].action[1]",
        "provenance names the exact property that requested the hop"
    );
    assert_eq!(
        request.provenance().chain(),
        std::slice::from_ref(&fx.source_path),
        "provenance carries the ledger's chain, not a fabricated one"
    );

    // The attempt harness may not commit identity. Nothing moved.
    assert_eq!(
        state.source_path, fx.source_path,
        "the dispatch must not swap the active document"
    );
    assert_eq!(
        state.entry,
        claudine::composition::DocumentEntryReason::Direct
    );
    assert!(
        guard.initialize_emitted() || guard.terminal_signal().is_some(),
        "the dispatch must not reset the guard for an uncommitted hand-off"
    );
}

/// A `proxy` to a nonexistent document is not the dispatch's problem to
/// detect: it has no filesystem authority. The request is well-formed and the
/// coordinator rejects it — see
/// [`super::coordinator_adoption::adopt_rejects_a_missing_target_without_activating_it`].
#[test]
fn dispatch_proxy_to_missing_target_still_only_requests() {
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
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );

    assert!(
        matches!(action, TerminalControlAction::Proxy(_)),
        "resolution is the coordinator's call, not the dispatch's; got {action:?}"
    );
    assert_eq!(
        state.source_path, fx.source_path,
        "the source document stays active until a hop is committed"
    );
}
