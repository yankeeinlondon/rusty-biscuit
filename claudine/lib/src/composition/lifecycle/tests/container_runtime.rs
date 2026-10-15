use super::*;
use crate::composition::LifecycleErrorInfo;
use crate::composition::lifecycle::runtime::{ContainerEventDecision as Decision, ContainerRunner};
use crate::composition::lifecycle::executor::{StackExecutionContext, StackControl, LifecycleEventOutcome};

fn exercise(frontmatter: serde_json::Value, run: impl FnOnce(&mut ContainerRunner<'_>, &StackExecutionContext<'_>, &RecordingEmitter)) {
    let config = parse_lifecycle_config(&frontmatter, dummy_path()).unwrap();
    let (settings, messaging, term) = test_ctx();
    let emitter = RecordingEmitter::new();
    let dir = tempfile::tempdir().unwrap();
    let ctx = LifecycleRuntimeContext {
        settings: &settings, messaging: &messaging, term: &term,
        source_path: dummy_path(), repo_root: None, launch_area: Some(dir.path()), context: None,
    };
    let engine = darkmatter::effects::EffectEngine::builder().mutation_root(dir.path()).auto_rehash(false).build();
    let map = frontmatter.as_object().unwrap();
    let stack = StackExecutionContext {
        seq: None, signal: LifecycleSignal::Initialize, scope: None,
        frontmatter: map, live_frontmatter: None, runtime_state: None,
        err: None, timing: None, current: None, group: None, loop_values: None,
        base_dir: Some(dir.path()), ctx_base_dir: Some(dir.path()), prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &engine, shell_runner: &crate::composition::lifecycle_executor::DisabledShellRunner,
        emitter: &emitter, term: &term, source_path: dummy_path(), repo_root: None,
        messaging: &messaging, settings: &settings,
    };
    let mut runner = ContainerRunner::new(make_guard(&config, &ctx, &emitter));
    run(&mut runner, &stack, &emitter);
    runner.guard_mut().defuse();
}

#[test]
fn container_retry_keeps_initialize_once_and_finalizes_only_exhausted_attempt() {
    exercise(json!({
        "initialize": {"stderr": "initialize"},
        "start": {"stderr": "start"},
        "failure": [{"stderr": "failure"}, {"retry": {"max_attempts": 1, "delay": "0s", "with": {"note": "second"}}}],
        "finalize": {"stderr": "finalize"}
    }), |runner, stack, emitter| {
        assert_eq!(runner.execute_event(LifecycleSignal::Initialize, stack).decision, Decision::Continue);
        assert_eq!(runner.execute_event(LifecycleSignal::Start, stack).decision, Decision::Continue);
        let first = runner.execute_event(LifecycleSignal::Failure, stack);
        let Decision::Retry { delay, overlay } = first.decision else { panic!("{first:?}") };
        assert!(delay.is_zero());
        assert_eq!(overlay.get("note"), Some(&json!("second")));
        assert_eq!(runner.attempt(), 2);
        runner.execute_event(LifecycleSignal::Initialize, stack);
        runner.execute_event(LifecycleSignal::Start, stack);
        assert!(matches!(runner.execute_event(LifecycleSignal::Failure, stack).decision, Decision::Finalize { failed: true, .. }));
        assert_eq!(runner.execute_event(LifecycleSignal::Finalize, stack).decision, Decision::Finished { failed: true });
        runner.execute_event(LifecycleSignal::Finalize, stack);
        let events: Vec<_> = emitter.signals();
        assert_eq!(events, [LifecycleSignal::Initialize, LifecycleSignal::Start, LifecycleSignal::Failure, LifecycleSignal::Start, LifecycleSignal::Failure, LifecycleSignal::Finalize]);
    });
}

#[test]
fn container_finalize_error_cannot_publish_success_and_keeps_original_error() {
    exercise(json!({"finalize": [{"error": "cleanup refused"}]}), |runner, stack, _| {
        runner.execute_event(LifecycleSignal::Initialize, stack);
        runner.execute_event(LifecycleSignal::Start, stack);
        assert!(matches!(runner.execute_event(LifecycleSignal::Success, stack).decision, Decision::Finalize { failed: false, .. }));
        assert_eq!(runner.execute_event(LifecycleSignal::Finalize, stack).decision, Decision::Finished { failed: true });
    });
    exercise(json!({"failure": {"stderr": "failed"}, "finalize": {"stderr": "done"}}), |runner, stack, _| {
        let error = LifecycleErrorInfo::from_action_failure("shell", "original child failure");
        runner.set_error(error.clone());
        assert_eq!(runner.execute_event(LifecycleSignal::Failure, stack).decision, Decision::Finalize { failed: true, error: Some(error) });
        assert_eq!(runner.execute_event(LifecycleSignal::Finalize, stack).decision, Decision::Finished { failed: true });
    });
}

#[test]
fn container_skip_has_no_cleanup_and_resume_has_no_session() {
    exercise(json!({"initialize": ["skip"], "finalize": {"stderr": "must not run"}}), |runner, stack, emitter| {
        assert_eq!(runner.execute_event(LifecycleSignal::Initialize, stack).decision, Decision::Skip);
        assert!(emitter.signals().is_empty());
        assert!(!runner.guard_mut().finalize_emitted());
    });
    exercise(json!({}), |runner, _, _| {
        let outcome = LifecycleEventOutcome { control: Some(StackControl::Resume { message: "again".into(), max_attempts: 1 }), ..Default::default() };
        assert_eq!(runner.record_outcome(LifecycleSignal::Failure, &outcome), Decision::Abort(crate::composition::lifecycle::runtime::LifecycleTransitionAbort::ResumeWithoutSession));
    });
}

#[test]
fn container_preflight_retry_is_refused_and_initialize_proxy_skips_cleanup() {
    exercise(json!({}), |runner, _, _| {
        let outcome = LifecycleEventOutcome { control: Some(StackControl::Retry { max_attempts: 1, backoff: crate::composition::lifecycle::actions::RetryBackoff::Fixed, delay: "0s".into(), overlay: Default::default() }), ..Default::default() };
        assert_eq!(runner.record_outcome(LifecycleSignal::Blocked, &outcome), Decision::SetupRecoveryUnsupported { event: LifecycleSignal::Blocked, action: "retry" });
    });
    exercise(json!({"initialize": [{"proxy": "@target.md"}], "finalize": {"stderr": "must not run"}}), |runner, stack, emitter| {
        assert!(matches!(runner.execute_event(LifecycleSignal::Initialize, stack).decision, Decision::Proxy { target, .. } if target == "@target.md"));
        assert!(emitter.signals().is_empty());
        assert!(!runner.guard_mut().finalize_emitted());
    });
}
