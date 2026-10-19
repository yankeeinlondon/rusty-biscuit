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

#[test]
fn cancellation_keeps_first_cause_and_refuses_finalize_recovery() {
    use crate::composition::lifecycle::cancellation::CancellationCause;
    exercise(json!({"initialize":{"stderr":"initialize"}, "start":{"stderr":"start"},
        "cancelled":[{"stderr":"cancelled"}],
        "finalize":[{"stderr":"finalize"},{"retry":{"max_attempts":1}}]}), |runner, stack, emitter| {
        runner.execute_event(LifecycleSignal::Initialize, stack);
        runner.execute_event(LifecycleSignal::Start, stack);
        assert!(runner.cancel(CancellationCause::Interrupt));
        assert!(!runner.cancel(CancellationCause::SiblingFailure {
            group:"later".into(), leg:"later".into(), error:LifecycleErrorInfo::from_action_failure("shell", "later failure"),
        }));
        let original = runner.primary_error().unwrap().clone();
        assert!(matches!(runner.execute_event(LifecycleSignal::Cancelled, stack).decision, Decision::Finalize { .. }));
        assert_eq!(runner.execute_event(LifecycleSignal::Finalize, stack).decision, Decision::Finished { failed:false });
        assert!(runner.is_cancelled());
        assert_eq!(runner.primary_error(), Some(&original));
        assert_eq!(runner.secondary_errors().len(), 1);
        assert!(runner.secondary_errors()[0].msg.contains("cannot recover a cancelled"));
        assert_eq!(runner.attempt(), 1);
        assert_eq!(emitter.signals(), [LifecycleSignal::Initialize, LifecycleSignal::Start,
            LifecycleSignal::Cancelled, LifecycleSignal::Finalize]);
        assert!(!runner.cancel(CancellationCause::Interrupt));
    });
}

#[test]
fn cancellation_before_start_runs_cleanup_with_shells_disabled() {
    exercise(json!({"cancelled":[{"shell":"must-not-run"}],
        "finalize":[{"stderr":"finalized"}]}), |runner, stack, emitter| {
        runner.execute_event(LifecycleSignal::Initialize, stack);
        runner.cancel(crate::composition::lifecycle::cancellation::CancellationCause::Interrupt);
        runner.execute_event(LifecycleSignal::Cancelled, stack);
        runner.execute_event(LifecycleSignal::Finalize, stack);
        assert!(runner.is_cancelled());
        assert_eq!(runner.secondary_errors().len(), 1);
        assert_eq!(emitter.signals(), [LifecycleSignal::Finalize]);
    });
}

#[test]
fn cancellation_refuses_every_recovery_and_retains_cleanup_timeout() {
    use crate::composition::lifecycle::cancellation::CancellationCause;
    let controls = [
        StackControl::Retry { max_attempts:1, backoff:crate::composition::lifecycle::actions::RetryBackoff::Fixed,
            delay:"0s".into(), overlay:Default::default() },
        StackControl::Resume {message:"again".into(), max_attempts:1},
        StackControl::Proxy {target:"must-not-load.md".into(), overlay:Default::default(),
            location:crate::composition::ActionLocation::new(LifecycleSignal::Finalize,
                crate::composition::lifecycle::actions::LifecyclePath::root("finalize"))},
        StackControl::Defer {delay:"1s".into(), reason:None},
    ];
    for control in controls {
        exercise(json!({}), |runner, stack, _| {
            runner.execute_event(LifecycleSignal::Initialize, stack);
            runner.cancel(CancellationCause::Interrupt);
            let original = runner.primary_error().unwrap().clone();
            let cleanup = LifecycleEventOutcome { action_error:Some(LifecycleErrorInfo::from_action_failure(
                "shell", "cleanup command timed out")), ..Default::default() };
            runner.record_outcome(LifecycleSignal::Cancelled, &cleanup);
            let recovery = LifecycleEventOutcome {control:Some(control), ..Default::default()};
            assert_eq!(runner.record_outcome(LifecycleSignal::Finalize, &recovery), Decision::Finished {failed:false});
            assert!(runner.is_cancelled());
            assert_eq!(runner.primary_error(), Some(&original));
            assert_eq!(runner.secondary_errors().len(), 2);
            assert!(runner.secondary_errors()[1].msg.contains("cannot recover a cancelled"));
            assert_eq!(runner.attempt(), 1);
        });
    }
}

#[test]
fn cancellation_scope_inherits_first_cause_and_restores_its_caller() {
    use crate::composition::lifecycle::cancellation::{self, CancellationCause, CancellationToken};
    let parent = CancellationToken::default();
    let child = parent.child();
    let original = CancellationCause::SiblingFailure { group:"checks".into(), leg:"lint".into(),
        error:LifecycleErrorInfo::from_action_failure("shell", "lint failed") };
    parent.cancel(original.clone());
    child.cancel(CancellationCause::Interrupt);
    {
        let _scope = child.enter();
        assert!(cancellation::requested());
        assert_eq!(cancellation::cause(), Some(original));
        let err = cancellation::cause().unwrap().error().to_value();
        assert_eq!(err["code"], "composition.cancelled");
        let declared: std::collections::BTreeSet<_> = crate::diagnostics::code_spec("composition.cancelled")
            .unwrap().detail.iter().copied().collect();
        let actual: std::collections::BTreeSet<_> = err["detail"].as_object().unwrap().keys()
            .map(String::as_str).collect();
        assert_eq!(actual, declared);
        assert_eq!(err["detail"]["group"], "checks");
        assert_eq!(err["detail"]["leg"], "lint");
        assert_eq!(err["detail"]["cause"]["msg"], "lint failed");
        {
            let _cleanup = cancellation::CleanupScope::enter();
            assert!(!cancellation::requested());
        }
        assert!(cancellation::requested());
    }
    assert!(!cancellation::requested());
}

#[test]
fn container_break_before_its_children_fires_only_finalize_once() {
    for event in ["initialize", "start"] {
        exercise(json!({
            event: ["break"],
            "success": {"stderr": "must not run"},
            "finalize": {"stderr": "finalize"}
        }), |runner, stack, emitter| {
            let ambient = crate::composition::LoopAmbient::new(1, true, false, json!([]), 0);
            let stack = StackExecutionContext { loop_values: Some(&ambient), ..stack.with_signal(stack.signal) };
            let mut signal = LifecycleSignal::Initialize;
            let finalize = loop {
                match runner.execute_event(signal, &stack).decision {
                    Decision::Continue => signal = LifecycleSignal::Start,
                    Decision::Finalize { failed: false, .. } => break runner.execute_event(LifecycleSignal::Finalize, &stack),
                    other => panic!("{event}: {other:?}"),
                }
            };
            assert_eq!(finalize.decision, Decision::Finished { failed: false }, "{event}");
            assert!(ambient.breaks.is_pending(), "{event}: the loop reads the break");
            assert_eq!(runner.execute_event(LifecycleSignal::Finalize, &stack).decision, Decision::Finished { failed: false });
            let fired: Vec<_> = emitter.signals().into_iter().filter(|signal| *signal == LifecycleSignal::Finalize).collect();
            assert_eq!(fired.len(), 1, "{event}: finalize runs exactly once");
            assert!(!emitter.signals().contains(&LifecycleSignal::Success), "{event}");
        });
    }
}
