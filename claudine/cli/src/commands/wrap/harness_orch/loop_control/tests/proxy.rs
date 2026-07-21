//! proxy harness-loop tests.

use super::*;

fn assert_proxy_detail(
    report: &color_eyre::eyre::Report,
    expected_event: &str,
    expected_property: &str,
) {
    let err = claudine::composition::LifecycleErrorInfo::from_error_or_action(
        "proxy",
        report.as_ref(),
    )
    .to_value();
    assert_eq!(
        err["code"],
        serde_json::json!("composition.invalid_file_reference")
    );
    assert_eq!(
        err["detail"]["event"],
        serde_json::json!(expected_event),
        "the structured event must not be inferred from the property: {}",
        err["detail"]
    );
    assert_eq!(
        err["detail"]["property"],
        serde_json::json!(expected_property),
        "the structured property must identify the authored proxy value: {}",
        err["detail"]
    );
}

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

#[test]
fn target_initialize_proxy_failure_projects_event_and_property_separately() {
    let fx = fixture(serde_json::json!({
        "initialize": {
            "stack": [{"action": {"proxy": "missing-target.md"}}]
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
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    let effect_engine = engine(fx._dir.path());

    let action = run_target_initialize(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &effect_engine,
        std::time::Instant::now(),
    );

    let TargetInitializeAction::Abort(report) = action else {
        panic!("expected the missing initialize proxy target to abort, got {action:?}");
    };
    assert_proxy_detail(&report, "initialize", "initialize.stack[*].proxy");
}

#[test]
fn terminal_proxy_failure_projects_event_and_property_separately() {
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
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    let mut budgets = ControlBudgets::default();
    let outcome = outcome_with(StackControl::Proxy {
        target: "missing-target.md".to_string(),
    });

    let action = dispatch_terminal_control(
        &outcome,
        1,
        &mut budgets,
        None,
        resume_capable_profile(),
        Provider::Claude,
        &mut state,
        &fx.materialized,
        Some(fx._dir.path()),
        &mut guard,
        &mut ProxyTracking::default(),
        &fx.term,
        false,
    );

    let TerminalControlAction::Abort(report) = action else {
        panic!("expected the missing terminal proxy target to abort, got {action:?}");
    };
    assert_proxy_detail(&report, "failure", "failure.stack[*].proxy");
}
