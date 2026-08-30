//! retry resume harness-loop tests.

use super::*;

#[derive(Default)]
struct CountingApprovalHandler {
    prompts: Mutex<Vec<String>>,
}

impl darkmatter::markdown::compose::shell_expansion::ShellApprovalHandler
    for CountingApprovalHandler
{
    fn approve(
        &self,
        request: darkmatter::markdown::compose::shell_expansion::ShellApprovalRequest,
    ) -> Result<
        darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision,
        darkmatter::markdown::compose::shell_expansion::ShellExpansionError,
    > {
        self.prompts.lock().unwrap().push(request.normalized_exact);
        Ok(darkmatter::markdown::compose::shell_expansion::ShellApprovalDecision::AllowOnce)
    }
}

fn run_production_attempt_preflight(
    state: &mut HarnessPromptState,
    harness_context: &mut CachedHarnessLoopContext,
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    effect_engine: &EffectEngine,
    term: &Terminal,
    child_cwd: &Path,
    attempt: u32,
) {
    let mut initial_materialized = None;
    let mut prompt = AttemptPromptPreparation {
        prompt_state: state,
        harness_context,
        initial_materialized: &mut initial_materialized,
        child_cwd,
        repo_root: Some(child_cwd),
        effective_non_interactive: true,
        show_checks: false,
        detail_requested: false,
        silent: true,
        stream_verbosity: claudine::stream::stderr::Verbosity::Silent,
    };
    let mut lifecycle = AttemptLifecycleExecution {
        guard,
        effect_engine,
        term,
        loop_start: loop_start_now(),
    };
    preflight_fresh_document_phase(&mut prompt, &mut lifecycle, attempt, false)
        .expect("fresh retry/resume document preflight succeeds");
}

#[test]
fn canonical_retry_and_resume_reentry_each_produce_exact_epoch_work() {
    let fx = fixture(serde_json::json!({}));
    std::fs::write(fx._dir.path().join("spec.md"), "launch-owned\n").unwrap();
    std::fs::write(
        &fx.source_path,
        "---\n$schema:\n  spec: 'file(eager; required)'\nprepared: '{{ ctx.os }}'\nprepared_cwd: '{{ ctx.cwd }}'\nprobe: \"$(echo {{ ctx.os }})\"\n\
         start:\n  stderr: '{{ ctx.os }} cwd={{ ctx.cwd }}'\n---\nbody={{ ctx.os }} cwd={{ ctx.cwd }} probe={{ probe }}\n",
    )
    .unwrap();
    let invocation =
        claudine::invocation_context::InvocationContext::capture_at(fx._dir.path());
    let mut state = prompt_state(&fx.source_path);
    state.source_context = Some(invocation.derive_source(&fx.source_path).unwrap());
    state.invocation_context = Some(invocation.clone());
    state.input_layers.set_overrides = Some(serde_json::json!({ "spec": "spec.md" }));
    state.input_layers.file_ref_fallback_dir = Some(fx._dir.path().to_path_buf());
    state.input_layers.file_resolution_context =
        Some(invocation.launch_file_resolution_context().clone());
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
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let approval_handler = std::sync::Arc::new(CountingApprovalHandler::default());
    let shell_options = claudine::harness::ShellApprovalOptions {
        policy_root: Some(fx._dir.path().to_path_buf()),
        approval_handler: Some(approval_handler.clone()),
        ..Default::default()
    };
    preflight_harness_document(&mut state, &shell_options, fx._dir.path())
        .expect("the initial document approves its shell-bearing context expression");
    let mut harness_context = CachedHarnessLoopContext::with_shell_options(
        &fx.source_path,
        Some(fx._dir.path()),
        shell_options,
    );
    harness_context.freeze_shell_approvals();
    let effect_engine = engine(fx._dir.path());
    let loop_start = loop_start_now();

    let retry = outcome_with(StackControl::Retry {
        max_attempts: 2,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    assert!(matches!(
        dispatch_terminal_control(
            &retry,
            1,
            active.iteration_mut(),
            Some("sess-1"),
            resume_capable_profile(),
            Provider::Claude,
            &mut state,
            &fx.materialized,
            &mut guard,
            &ledger(&fx.source_path),
            &fx.term,
            false,
        ),
        TerminalControlAction::Continue
    ));
    assert_eq!(state.entry, claudine::composition::DocumentEntryReason::Retry);
    run_production_attempt_preflight(
        &mut state,
        &mut harness_context,
        &mut guard,
        &effect_engine,
        &fx.term,
        fx._dir.path(),
        active.iteration().attempt().number(),
    );
    let retried = materialize_harness_prompt(
        &mut state,
        Some(fx._dir.path()),
        fx._dir.path(),
        None,
        claudine::composition::SchemaStage::Validate,
    )
    .unwrap();
    assert!(retried.prompt.contains("body="));
    let expected_cwd = biscuit_file::to_portable_string(fx._dir.path());
    assert!(retried.prompt.contains(&format!("cwd={expected_cwd}")));
    assert_eq!(retried.frontmatter["prepared_cwd"], serde_json::json!(expected_cwd));
    // D8.4: an eager file() materializes the winning absolute NATIVE path —
    // unlike ctx.cwd above, this value is never portable-converted.
    let expected_spec = fx._dir.path().join("spec.md").display().to_string();
    assert_eq!(retried.frontmatter["spec"], serde_json::json!(expected_spec));
    guard.set_config(retried.lifecycle.clone().unwrap());
    let start = run_start_lifecycle_event(
        &state,
        &mut guard,
        &retried,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &effect_engine,
        None,
        loop_start,
    );
    assert!(start.evaluation_error.is_none());
    assert_eq!(
        retried.document_epoch.as_ref().unwrap().work_snapshot(),
        claudine::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: std::collections::BTreeMap::from([
                ("body".to_string(), 1),
                ("effective-frontmatter".to_string(), 1),
                ("lifecycle".to_string(), 1),
                ("preflight".to_string(), 1),
            ]),
        },
        "the retry transition must lead to one fresh canonical epoch"
    );

    guard.mark_provider_launched();
    let resume = outcome_with(StackControl::Resume {
        message: "continue".to_string(),
        max_attempts: 1,
    });
    assert!(matches!(
        dispatch_terminal_control(
            &resume,
            2,
            active.iteration_mut(),
            Some("sess-2"),
            resume_capable_profile(),
            Provider::Claude,
            &mut state,
            &retried,
            &mut guard,
            &ledger(&fx.source_path),
            &fx.term,
            false,
        ),
        TerminalControlAction::Continue
    ));
    assert_eq!(state.entry, claudine::composition::DocumentEntryReason::Resume);
    run_production_attempt_preflight(
        &mut state,
        &mut harness_context,
        &mut guard,
        &effect_engine,
        &fx.term,
        fx._dir.path(),
        active.iteration().attempt().number(),
    );
    let resumed = materialize_harness_prompt(
        &mut state,
        Some(fx._dir.path()),
        fx._dir.path(),
        active.iteration().attempt().resume_followup(),
        claudine::composition::SchemaStage::Validate,
    )
    .unwrap();
    assert_eq!(resumed.prompt, "continue");
    assert_eq!(resumed.frontmatter["prepared_cwd"], serde_json::json!(expected_cwd));
    assert_eq!(resumed.frontmatter["spec"], serde_json::json!(expected_spec));
    guard.set_config(resumed.lifecycle.clone().unwrap());
    let start = run_start_lifecycle_event(
        &state,
        &mut guard,
        &resumed,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &effect_engine,
        None,
        loop_start,
    );
    assert!(start.evaluation_error.is_none());
    assert_eq!(
        resumed.document_epoch.as_ref().unwrap().work_snapshot(),
        claudine::invocation_context::DocumentEpochWork {
            launch_context_constructions: 1,
            launch_context_extensions: 0,
            ambient_fallbacks: 0,
            prepared_context_consumers: std::collections::BTreeMap::from([
                ("body".to_string(), 1),
                ("effective-frontmatter".to_string(), 1),
                ("lifecycle".to_string(), 1),
                ("preflight".to_string(), 1),
            ]),
        },
        "the resume transition must lead to one fresh canonical epoch"
    );
    assert_eq!(
        approval_handler.prompts.lock().unwrap().len(),
        1,
        "retry and resume keep the approval window frozen and reuse the initial approval"
    );
}

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
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 2,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    let action = dispatch_terminal_control(
        &outcome,
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
    match action {
        TerminalControlAction::Continue => assert_eq!(active.iteration().attempt().number(), 2),
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
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 1,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    let action = dispatch_terminal_control(
        &outcome,
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
    match action {
        TerminalControlAction::Continue => assert_eq!(active.iteration().attempt().number(), 2),
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
    let mut active = claudine::composition::ActiveDocumentState::initial();

    let outcome = outcome_with(StackControl::Resume {
        message: "finish the task".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
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
    assert!(matches!(action, TerminalControlAction::Continue));
    // The resume directive is recorded on the freshly-advanced provider-attempt
    // slice, not on a parallel prompt-state field.
    assert_eq!(active.iteration().attempt().number(), 2);
    assert_eq!(
        active.iteration().attempt().resume_followup(),
        Some("finish the task")
    );
    assert_eq!(active.iteration().attempt().session_id(), Some("sess-1"));
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
    let mut active = claudine::composition::ActiveDocumentState::initial();
    active.iteration_mut().retry_budget_mut().ceiling_for(1, 1);
    let outcome = outcome_with(StackControl::Retry {
        max_attempts: 1,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    // attempt 2 has reached the ceiling → fall through (no continue).
    let action = dispatch_terminal_control(
        &outcome,
        2,
        active.iteration_mut(),
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
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let outcome = outcome_with(StackControl::Resume {
        message: "please finish the task".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        active.iteration_mut(),
        Some("sess-42"),
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
    assert_eq!(active.iteration().attempt().session_id(), Some("sess-42"));
    assert_eq!(
        active.iteration().attempt().resume_followup(),
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
    let mut active = claudine::composition::ActiveDocumentState::initial();
    let outcome = outcome_with(StackControl::Resume {
        message: "x".to_string(),
        max_attempts: 1,
    });
    let action = dispatch_terminal_control(
        &outcome,
        1,
        active.iteration_mut(),
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
            assert!(
                err.to_string().contains("requires a live provider session"),
                "unexpected: {err}"
            );
        }
        other => panic!("expected Abort, got {other:?}"),
    }
}

/// A resume refused by the session-compatibility comparison is a post-`start`,
/// pre-spawn failure, so it owes the ratified lifecycle tail: `failure` then
/// exactly one `finalize`, both able to observe the incompatibility as `err.*`.
///
/// The guard is put in the state a resumed attempt actually reaches — `start`
/// spent, the previous iteration's terminal already reset — so the assertion is
/// about the routing decision, not about a hand-built ledger.
#[test]
fn a_refused_resume_routes_through_failure_then_finalize_with_err() {
    let fx = fixture(serde_json::json!({
        "failure": {
            "stack": [
                {"when": "err", "action": {"append_line": ["events.log", "{{ err.msg }}"]}}
            ]
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
    // The resumed attempt re-entered at `start`; the opening attempt's terminal
    // was reset by `reset_for_next_iteration`, so the slot is free again.
    assert!(guard.record_event_emission(LifecycleSignal::Start));

    let report = route_incompatible_resume(
        &mut guard,
        &fx.materialized,
        &fx.source_path,
        Some(fx._dir.path()),
        &fx.term,
        &eng,
        CompositionError::LifecycleResumeIncompatible {
            source_path: fx.source_path.clone(),
            facets: vec!["model".to_string()],
        },
        std::time::Instant::now(),
    );

    assert_eq!(
        guard.terminal_signal(),
        Some(LifecycleSignal::Failure),
        "a post-`start` refusal is a `failure`, never a pre-flight `blocked`"
    );
    assert!(guard.finalize_emitted(), "the owed `finalize` fired");
    let log = std::fs::read_to_string(&fx.log_path).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "each stack runs exactly once; got {lines:?}"
    );
    assert!(
        lines[0].contains("cannot reuse the live session") && lines[0].contains("model"),
        "`failure` observes the refusal itself as `err`, naming the changed facet; got {:?}",
        lines[0]
    );
    assert_eq!(lines[1], "finalize-ran", "`finalize` saw `err` populated");
    assert!(
        report
            .downcast_ref::<CompositionError>()
            .is_some_and(|e| matches!(e, CompositionError::LifecycleResumeIncompatible { .. })),
        "the incompatibility stays the active error rather than being replaced"
    );
}
