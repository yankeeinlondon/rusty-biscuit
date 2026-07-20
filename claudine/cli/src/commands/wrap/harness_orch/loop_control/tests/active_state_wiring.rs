//! The active-document state model is the runtime's single source of truth.
//!
//! These tests exercise the *wired* runtime, not the isolated library type: a
//! dispatch records the provider-attempt slice on the one `ActiveDocumentState`
//! the loop owns, and the downstream read paths — prompt materialization and
//! launch planning — consult that same model rather than a parallel prompt-state
//! field. If the wiring regressed to two sources of truth, the recorded value
//! and the value the runtime acts on would diverge and these would fail.

use super::*;

use claudine::composition::ActiveDocumentState;
use std::collections::HashMap;
use std::ffi::OsString;

fn runtime_ctx<'a>(fx: &'a Fixture) -> LifecycleRuntimeContext<'a> {
    LifecycleRuntimeContext {
        settings: &fx.settings,
        messaging: &fx.messaging,
        term: &fx.term,
        source_path: &fx.source_path,
        repo_root: Some(fx._dir.path()),
        launch_area: None,
        context: None,
    }
}

/// A materialized prompt with a real body, for launch-planning tests whose
/// prompt-present check would reject the empty seed.
fn materialized_with_prompt(prompt: &str) -> MaterializedHarnessPrompt {
    MaterializedHarnessPrompt {
        frontmatter: serde_json::Value::Null,
        prompt: prompt.to_string(),
        env_overrides: Vec::new(),
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
        inline_closure_plan: None,
        lifecycle: None,
        live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&serde_json::Value::Null),
        mcp_body_tags: Vec::new(),
    }
}

/// A resume records its follow-up on the model's attempt slice, and prompt
/// materialization — the runtime's read path — substitutes that follow-up for
/// the composed body. This is the end-to-end proof that `resume` no longer
/// stores its directive in a parallel `HarnessPromptState` field.
#[test]
fn a_resume_followup_recorded_on_the_model_overrides_the_composed_body() {
    let fx = fixture(serde_json::json!({}));
    // The composed body is deliberately *not* the follow-up, so a prompt equal
    // to the follow-up can only have come from the model.
    std::fs::write(&fx.source_path, "---\ndescription: d\n---\nORIGINAL COMPOSED BODY\n").unwrap();
    let emitter = RecordingEmitter::default();
    let ctx = runtime_ctx(&fx);
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    let mut state = prompt_state(&fx.source_path);
    let mut active = ActiveDocumentState::initial();

    let outcome = outcome_with(StackControl::Resume {
        message: "RESUME FOLLOW-UP INPUT".to_string(),
        max_attempts: 2,
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
    assert!(matches!(action, TerminalControlAction::Continue));

    // Exactly the argument `materialize_attempt_prompt_phase` threads: the
    // follow-up recorded on the model's freshly-advanced attempt slice.
    let materialized = super::super::super::materialize_harness_prompt(
        &state,
        Some(fx._dir.path()),
        fx._dir.path(),
        active.iteration().attempt().resume_followup(),
        claudine::composition::SchemaStage::Validate,
    )
    .expect("the resumed attempt materializes");
    assert_eq!(
        materialized.prompt.trim(),
        "RESUME FOLLOW-UP INPUT",
        "the runtime read the follow-up from the model, not the composed body",
    );
}

/// The live session recorded by a resume drives the launch's resume arguments;
/// a fresh attempt (no session) launches against the base arguments. The launch
/// planner reads the session from the caller, which reads it from the model.
#[test]
fn the_model_session_decides_whether_the_launch_resumes() {
    let base_args = vec!["--print".to_string()];
    let base_env: HashMap<OsString, OsString> = HashMap::new();
    let materialized = materialized_with_prompt("do the work");

    // No live session → the base arguments launch a fresh provider session, and
    // the resume id never appears.
    let fresh = super::super::super::build_harness_launch(
        Provider::Claude,
        resume_capable_profile(),
        &base_args,
        &base_env,
        None,
        &materialized,
        &[],
        true,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("a fresh launch plan builds");
    assert!(
        !fresh.args.iter().any(|arg| arg == "sess-77"),
        "a fresh attempt does not resume any session: {:?}",
        fresh.args,
    );

    // A live session on the model → the launch carries the resume id.
    let resumed = super::super::super::build_harness_launch(
        Provider::Claude,
        resume_capable_profile(),
        &base_args,
        &base_env,
        Some("sess-77"),
        &materialized,
        &[],
        true,
        None,
        None,
        None,
        None,
        None,
    )
    .expect("a resuming launch plan builds");
    assert!(
        resumed.args.iter().any(|arg| arg == "sess-77"),
        "the model's live session id drives the resume arguments: {:?}",
        resumed.args,
    );
}

/// A retry after a resume replaces the provider-attempt slice with a fresh one,
/// dropping the live session — so the retried attempt launches a fresh session
/// even though the resume before it had one. Retaining it would silently resume
/// a session a retry is supposed to abandon.
#[test]
fn a_retry_after_a_resume_drops_the_live_session() {
    let fx = fixture(serde_json::json!({}));
    let emitter = RecordingEmitter::default();
    let ctx = runtime_ctx(&fx);
    let mut guard = dispatch_guard(&fx.config, &ctx, &emitter);
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let mut state = prompt_state(&fx.source_path);
    let mut active = ActiveDocumentState::initial();

    // A resume adopts the live session onto the new attempt slice.
    let resume = outcome_with(StackControl::Resume {
        message: "keep going".to_string(),
        max_attempts: 3,
    });
    dispatch_terminal_control(
        &resume,
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
    assert_eq!(active.iteration().attempt().session_id(), Some("sess-1"));

    // Re-arm the guard exactly as the loop's re-entry does, then a retry fires.
    guard.mark_provider_launched();
    assert!(guard.record_event_emission(LifecycleSignal::Failure));
    let retry = outcome_with(StackControl::Retry {
        max_attempts: 3,
        backoff: RetryBackoff::Fixed,
        delay: "0s".to_string(),
    });
    dispatch_terminal_control(
        &retry,
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

    assert_eq!(
        active.iteration().attempt().session_id(),
        None,
        "a retry starts a fresh provider session, dropping the resumed one",
    );
    assert_eq!(
        active.iteration().attempt().resume_followup(),
        None,
        "a retry carries no follow-up override",
    );
    // The enclosing iteration's budgets survived both replacements.
    assert!(
        active.iteration().resume_budget().ceiling().is_some(),
        "the resume ceiling earned by the first resume is retained across the retry",
    );
}
