//! Retry/resume budgets are scoped to the active document iteration.
//!
//! A budget is the absolute attempt ceiling a `retry`/`resume` control earned
//! when it first fired, so it belongs to the document that declared it. A proxy
//! hands the run to a *different* document, whose own `max_attempts` must earn a
//! fresh ceiling. Invocation-wide hop/cycle accounting is the opposite: it must
//! survive the same boundary, or `A → B → A` stops being a cycle.

use super::*;

use claudine::composition::{EvaluatedProxyRequest, ProxyProvenance};

struct BudgetFixture {
    fx: Fixture,
    target: PathBuf,
}

/// Both documents on disk: the coordinator resolves through the shared
/// resolver, which checks existence, so a budget test needs real files rather
/// than a resolution failure standing in for the hop.
fn budget_fixture() -> BudgetFixture {
    let fx = fixture(serde_json::json!({}));
    std::fs::write(&fx.source_path, "---\n---\nbody\n").unwrap();
    let target = fx._dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody\n").unwrap();
    BudgetFixture { fx, target }
}

fn request_for(source: &Path, target: &Path, chain: Vec<PathBuf>) -> EvaluatedProxyRequest {
    EvaluatedProxyRequest::new(
        target.display().to_string(),
        indexmap::IndexMap::new(),
        ProxyProvenance::new(
            source.to_path_buf(),
            claudine::composition::ActionLocation::new(LifecycleSignal::Failure, 0, 0),
            chain,
        ),
    )
}

fn retry(max_attempts: u32) -> LifecycleEventOutcome {
    outcome_with(StackControl::Retry {
        max_attempts,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    })
}

fn resume(max_attempts: u32) -> LifecycleEventOutcome {
    outcome_with(StackControl::Resume {
        message: "finish the task".to_string(),
        max_attempts,
    })
}

/// Re-arm the guard so the next attempt's terminal event can fire, exactly as
/// the loop's own re-entry does.
fn arm_failure(guard: &mut claudine::composition::LifecycleRunGuard<'_>) {
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
}

#[test]
fn a_proxy_target_earns_its_own_retry_budget_rather_than_inheriting_the_sources() {
    let BudgetFixture { fx, target } = budget_fixture();
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
    arm_failure(&mut guard);
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let mut coord = coordinator(&fx.source_path);

    // The source's `retry: {max_attempts: 1}` fires at attempt 1, earning
    // ceiling 2.
    let action = dispatch_terminal_control(
        &retry(1),
        1,
        active.iteration_mut(),
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        coord.ledger(),
        &fx.term,
        false,
    );
    assert!(
        matches!(action, TerminalControlAction::Continue),
        "the source's own retry is honored"
    );
    assert_eq!(active.iteration().retry_budget().ceiling(), Some(2), "the source earned ceiling 2");

    // The source hands off at attempt 2.
    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut active,
        )
        .expect("the hop is resolvable and uncontested");
    assert_eq!(
        active.iteration().retry_budget().ceiling(), None,
        "the source's ceiling did not follow the run to the target"
    );

    // The target's own `retry: {max_attempts: 3}` fires at attempt 2. Against
    // the source's stale ceiling of 2 it is already exhausted; against its own
    // it has three attempts to spend.
    arm_failure(&mut guard);
    let action = dispatch_terminal_control(
        &retry(3),
        2,
        active.iteration_mut(),
        Some("sess-2"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        coord.ledger(),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Continue => {},
        other => panic!("the target's own retry budget must be honored, got {other:?}"),
    }
    assert_eq!(
        active.iteration().retry_budget().ceiling(),
        Some(5),
        "the ceiling is the target's max_attempts measured from attempt 2"
    );
}

#[test]
fn a_proxy_target_earns_its_own_resume_budget_rather_than_inheriting_the_sources() {
    let BudgetFixture { fx, target } = budget_fixture();
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
    arm_failure(&mut guard);
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let mut coord = coordinator(&fx.source_path);

    let action = dispatch_terminal_control(
        &resume(1),
        1,
        active.iteration_mut(),
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        coord.ledger(),
        &fx.term,
        false,
    );
    assert!(
        matches!(action, TerminalControlAction::Continue),
        "the source's own resume is honored"
    );
    assert_eq!(active.iteration().resume_budget().ceiling(), Some(2), "the source earned ceiling 2");

    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut active,
        )
        .expect("the hop is resolvable and uncontested");
    assert_eq!(
        active.iteration().resume_budget().ceiling(), None,
        "the source's ceiling did not follow the run to the target"
    );

    arm_failure(&mut guard);
    let action = dispatch_terminal_control(
        &resume(3),
        2,
        active.iteration_mut(),
        Some("sess-2"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        coord.ledger(),
        &fx.term,
        false,
    );
    match action {
        TerminalControlAction::Continue => {},
        other => panic!("the target's own resume budget must be honored, got {other:?}"),
    }
    assert_eq!(active.iteration().resume_budget().ceiling(), Some(5));
}

#[test]
fn adoption_resets_budgets_while_the_invocation_wide_chain_keeps_growing() {
    // The two halves of the same rule: the budget is document-scoped and dies
    // at the boundary; hop/cycle accounting is invocation-wide and must not.
    let BudgetFixture { fx, target } = budget_fixture();
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
    arm_failure(&mut guard);
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    active.iteration_mut().retry_budget_mut().ceiling_for(1, 1); // ceiling 2
    active.iteration_mut().resume_budget_mut().ceiling_for(1, 3); // ceiling 4
    let mut coord = coordinator(&fx.source_path);
    assert_eq!(
        coord.ledger().chain(),
        std::slice::from_ref(&fx.source_path),
        "the chain originates at the source"
    );

    coord
        .adopt(
            request_for(&fx.source_path, &target, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut active,
        )
        .expect("the hop is resolvable and uncontested");

    assert_eq!(active.iteration().retry_budget().ceiling(), None, "retry ceiling is document-scoped");
    assert_eq!(active.iteration().resume_budget().ceiling(), None, "resume ceiling is document-scoped");
    assert_eq!(
        coord.ledger().chain(),
        &[fx.source_path.clone(), target.clone()],
        "hop accounting is invocation-wide and survives the reset"
    );
}

#[test]
fn a_refused_hop_leaves_the_budgets_the_source_earned_intact() {
    // A refused hand-off never half-activates a target — and must not
    // half-retire the source either: the source is still the active document,
    // so the ceiling it earned still governs it.
    let BudgetFixture { fx, target: _ } = budget_fixture();
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
    arm_failure(&mut guard);
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();
    active.iteration_mut().retry_budget_mut().ceiling_for(1, 1); // ceiling 2
    active.iteration_mut().resume_budget_mut().ceiling_for(1, 3); // ceiling 4
    let mut coord = coordinator(&fx.source_path);

    let missing = fx._dir.path().join("does-not-exist.md");
    coord
        .adopt(
            request_for(&fx.source_path, &missing, vec![fx.source_path.clone()]),
            Some(fx._dir.path()),
            &mut state,
            &mut guard,
            &mut active,
        )
        .expect_err("an unresolvable target is refused");

    assert_eq!(active.iteration().retry_budget().ceiling(), Some(2), "the source's ceiling is untouched");
    assert_eq!(active.iteration().resume_budget().ceiling(), Some(4), "the source's ceiling is untouched");
    assert_eq!(
        state.source_path, fx.source_path,
        "the source is still the active document"
    );
}

#[test]
fn a_retry_cannot_reset_its_own_ceiling_by_firing_again() {
    // Replacing the provider-attempt slice retains and *decrements* the
    // budget. If re-firing re-derived the ceiling from the new attempt, a
    // `retry: {max_attempts: 1}` would loop forever.
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
    arm_failure(&mut guard);
    let mut state = prompt_state(&fx.source_path);
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let action = dispatch_terminal_control(
        &retry(1),
        1,
        active.iteration_mut(),
        Some("sess-1"),
        resume_capable_profile(),
        Provider::Goose,
        &mut state,
        &fx.materialized,
        &mut guard,
        &ledger(&fx.source_path),
        &fx.term,
        false,
    );
    assert!(matches!(
        action,
        TerminalControlAction::Continue
    ));

    // The same control fires again at attempt 2. Re-derivation would give
    // ceiling 3; the retained ceiling of 2 is exhausted.
    arm_failure(&mut guard);
    let action = dispatch_terminal_control(
        &retry(1),
        2,
        active.iteration_mut(),
        Some("sess-1"),
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
        matches!(action, TerminalControlAction::Fallthrough),
        "the retained ceiling is exhausted; the retry does not re-arm itself"
    );
    assert_eq!(active.iteration().retry_budget().ceiling(), Some(2), "the ceiling never moved");
}
