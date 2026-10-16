//! THROWAWAY Phase 1 spike for 2026-10-25-loops-and-flow-control.
//!
//! Drives the real `ContainerRunner`/`LifecycleRunGuard` with fake primitive
//! bodies to test the interface contracts `break`, `prep`, and per-pass retry
//! need. Removed after one run; archived in the feature's `spike/` directory.

use super::*;
use crate::composition::lifecycle::executor::StackExecutionContext;
use crate::composition::lifecycle::runtime::{ContainerEventDecision as Decision, ContainerRunner};
use crate::composition::lifecycle_executor::{ShellRunError, ShellRunner};
use crate::composition::runtime_state::RuntimeState;
use darkmatter::markdown::compose::shell_expansion::template::FilledChain;
use serde_json::Value;

/// Identity of the loop frame a `break` targets; allocated before `initialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoopId(u32);

/// What one primitive body (fake) reports to its owner.
#[derive(Debug, Clone, PartialEq)]
enum BodyOutcome {
    Completed(Value),
    Broken { target: LoopId, reason: Option<String>, code: u8 },
    Failed(String),
}

/// What a container reports after its own cleanup settled.
#[derive(Debug, Clone, PartialEq)]
enum Settled {
    Completed(Value),
    /// The break still travels toward `target`; `partial` is published once.
    Broken { target: LoopId, reason: Option<String>, code: u8, partial: Value },
    Failed(String),
}

fn with_stack(
    frontmatter: Value,
    shell: &dyn ShellRunner,
    run: impl FnOnce(&LifecycleConfig, &LifecycleRuntimeContext<'_>, &StackExecutionContext<'_>, &RecordingEmitter),
) {
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
        effect_engine: &engine, shell_runner: shell,
        emitter: &emitter, term: &term, source_path: dummy_path(), repo_root: None,
        messaging: &messaging, settings: &settings,
    };
    run(&config, &ctx, &stack, &emitter);
}

/// Container driver under test: run initialize/start, the body, then either
/// the terminal event or (for a break) cleanup only.
fn run_container(
    runner: &mut ContainerRunner<'_>,
    stack: &StackExecutionContext<'_>,
    body: impl FnOnce() -> BodyOutcome,
) -> Settled {
    assert_eq!(runner.execute_event(LifecycleSignal::Initialize, stack).decision, Decision::Continue);
    assert_eq!(runner.execute_event(LifecycleSignal::Start, stack).decision, Decision::Continue);
    let outcome = body();
    if let BodyOutcome::Broken { .. } = &outcome {
        // No terminal event fires between a break and its loop; only cleanup.
        // `defuse` marks the terminal slot settled so `finalize` may run once
        // and Drop does not synthesize `failure`/`blocked`.
        runner.guard_mut().defuse();
    } else {
        let signal = if matches!(outcome, BodyOutcome::Failed(_)) { LifecycleSignal::Failure } else { LifecycleSignal::Success };
        if let BodyOutcome::Failed(message) = &outcome {
            runner.set_error(crate::composition::LifecycleErrorInfo::from_action_failure("body", message.clone()));
        }
        let terminal = runner.execute_event(signal, stack);
        assert!(matches!(terminal.decision, Decision::Finalize { .. }), "{terminal:?}");
    }
    let finalize = runner.execute_event(LifecycleSignal::Finalize, stack);
    let Decision::Finished { failed } = finalize.decision else { panic!("{finalize:?}") };
    // A second finalize request is ignored by the guard.
    let again = runner.execute_event(LifecycleSignal::Finalize, stack);
    assert_eq!(again.decision, Decision::Finished { failed });
    match outcome {
        // Failed cleanup defeats a pending break.
        _ if failed && runner.primary_error().is_some() =>
            Settled::Failed(runner.primary_error().unwrap().msg.clone()),
        BodyOutcome::Completed(value) => Settled::Completed(value),
        BodyOutcome::Broken { target, reason, code } =>
            Settled::Broken { target, reason, code, partial: json!(["partial"]) },
        BodyOutcome::Failed(message) => Settled::Failed(message),
    }
}

fn stderr_texts(emitter: &RecordingEmitter) -> Vec<String> {
    emitter.actions().into_iter().filter_map(|action| match action {
        EmittedAction::Stderr { text, .. } => Some(text),
        _ => None,
    }).collect()
}

#[test]
fn spike_break_crosses_a_container_with_exactly_once_cleanup() {
    let outer_loop = LoopId(1);
    with_stack(json!({
        "initialize": {"stderr": "group initialize"},
        "start": {"stderr": "group start"},
        "success": {"stderr": "group success must not run"},
        "finalize": {"stderr": "group finalize"}
    }), &crate::composition::lifecycle_executor::DisabledShellRunner, |config, ctx, stack, emitter| {
        let mut runner = ContainerRunner::new(LifecycleRunGuard::new(config, ctx, emitter));
        let settled = run_container(&mut runner, stack, || BodyOutcome::Broken {
            target: outer_loop, reason: Some("ready".into()), code: 0,
        });
        assert_eq!(settled, Settled::Broken { target: outer_loop, reason: Some("ready".into()), code: 0, partial: json!(["partial"]) });
        assert_eq!(stderr_texts(emitter), ["group initialize", "group start", "group finalize"]);
        drop(runner);
        // Drop did not synthesize a terminal event after the break.
        assert_eq!(stderr_texts(emitter).len(), 3);
    });
}

#[test]
fn spike_failed_cleanup_defeats_a_pending_break() {
    with_stack(json!({"finalize": [{"stderr": "cleanup"}, {"error": "cleanup refused"}]}),
        &crate::composition::lifecycle_executor::DisabledShellRunner, |config, ctx, stack, emitter| {
        let mut runner = ContainerRunner::new(LifecycleRunGuard::new(config, ctx, emitter));
        let settled = run_container(&mut runner, stack, || BodyOutcome::Broken { target: LoopId(1), reason: None, code: 2 });
        assert!(matches!(settled, Settled::Failed(ref message) if message.contains("cleanup refused")), "{settled:?}");
        assert_eq!(stderr_texts(emitter), ["cleanup"]);
    });
}

/// One leg's settled result, in declaration order.
#[derive(Debug, Clone)]
struct LegSettlement { name: &'static str, settled: Settled }

/// Group pass reduction: every leg has settled before this runs. A pending
/// break selects the first breaking leg in declaration order; an unhandled
/// failure (or any failure when `fail_fast`) wins over it.
fn settle_group_pass(legs: &[LegSettlement], fail_fast: bool) -> Settled {
    let mut result = serde_json::Map::new();
    let mut breaking = None;
    for leg in legs {
        match &leg.settled {
            Settled::Failed(message) if fail_fast => return Settled::Failed(format!("{}: {message}", leg.name)),
            Settled::Failed(_) => { result.insert(leg.name.into(), Value::Null); }
            Settled::Completed(value) => { result.insert(leg.name.into(), value.clone()); }
            Settled::Broken { target, reason, code, partial } => {
                result.insert(leg.name.into(), partial.clone());
                breaking.get_or_insert((*target, reason.clone(), *code));
            }
        }
    }
    match breaking {
        Some((target, reason, code)) => Settled::Broken { target, reason, code, partial: Value::Object(result) },
        None => Settled::Completed(Value::Object(result)),
    }
}

#[test]
fn spike_pending_concurrent_break_waits_and_picks_declaration_order() {
    let target = LoopId(7);
    // Completion order is c, b, a; declaration order is a, b, c.
    let completion_order = ["c", "b", "a"];
    let mut by_name = std::collections::HashMap::new();
    for name in completion_order {
        let settled = match name {
            "a" => Settled::Completed(json!("A")),
            "b" => Settled::Broken { target, reason: Some("from b".into()), code: 0, partial: json!("B") },
            _ => Settled::Broken { target, reason: Some("from c".into()), code: 3, partial: json!("C") },
        };
        by_name.insert(name, settled);
    }
    let legs: Vec<_> = ["a", "b", "c"].into_iter().map(|name| LegSettlement { name, settled: by_name[name].clone() }).collect();
    assert_eq!(settle_group_pass(&legs, false), Settled::Broken {
        target, reason: Some("from b".into()), code: 0, partial: json!({"a": "A", "b": "B", "c": "C"}),
    });
    // A handled leg failure keeps the pending break; fail_fast does not.
    let mut with_failure = legs.clone();
    with_failure[0].settled = Settled::Failed("boom".into());
    assert!(matches!(settle_group_pass(&with_failure, false), Settled::Broken { .. }));
    assert_eq!(settle_group_pass(&with_failure, true), Settled::Failed("a: boom".into()));
}

/// Stand-in for a returning `prep` target runner: called inline by an action
/// and returning to the stack, which continues with the next item.
struct RecordingCall { calls: std::sync::Mutex<Vec<String>> }

impl ShellRunner for RecordingCall {
    fn run(&self, command: &str) -> Result<i32, ShellRunError> {
        self.calls.lock().unwrap().push(command.into());
        Ok(0)
    }
    fn run_structured(&self, _chain: &FilledChain) -> Result<i32, ShellRunError> { Ok(0) }
}

#[test]
fn spike_returning_action_continues_the_list_and_after_the_branch() {
    let call = RecordingCall { calls: Default::default() };
    with_stack(json!({
        "start": [
            {"shell": "call-helper-1"},
            {"stderr": "after call"},
            {"when": "true", "then": [{"shell": "call-helper-2"}]},
            {"stderr": "after branch"}
        ]
    }), &call, |config, ctx, stack, emitter| {
        let mut guard = LifecycleRunGuard::new(config, ctx, emitter);
        let outcome = guard.execute_event(LifecycleSignal::Start, &stack.with_signal(LifecycleSignal::Start));
        assert!(outcome.control.is_none() && outcome.action_error.is_none() && outcome.evaluation_error.is_none(), "{outcome:?}");
        assert_eq!(*call.calls.lock().unwrap(), ["call-helper-1", "call-helper-2"]);
        assert_eq!(stderr_texts(emitter), ["after call", "after branch"]);
        guard.defuse();
    });
}

#[test]
fn spike_retry_restores_only_the_current_pass() {
    let engine = darkmatter::effects::EffectEngine::builder().auto_rehash(false).build();
    let state = RuntimeState::new();
    let base = serde_json::Map::new();
    // Pass 1 completes A, B and writes k = 1.
    state.append_output("A");
    state.append_output("B");
    state.set(&engine, "k", json!(1), &base).unwrap();
    let checkpoint = state.snapshot();
    // Pass 2 attempt 1: A succeeds, k = 2, then B fails and retries.
    state.append_output("A");
    state.set(&engine, "k", json!(2), &base).unwrap();
    // Outputs can be restored with the existing seam...
    state.replace_sequence_scope(checkpoint.seq.clone(), checkpoint.outputs.clone());
    assert_eq!(state.outputs_value(), json!(["A", "B"]));
    // ...but the mutation layer keeps the failed attempt's write: a new
    // checkpoint-restore seam is required (finding, not a production change).
    assert_eq!(state.snapshot().mutations.get("k"), Some(&json!(2)));
    // A fresh cell built from the checkpoint has exactly pass-2 entry state.
    let restored = RuntimeState::from_snapshot(checkpoint);
    assert_eq!(restored.snapshot().mutations.get("k"), Some(&json!(1)));
    assert_eq!(restored.outputs_value(), json!(["A", "B"]));
}

/// Spike-only deadline carrier: an absolute instant, never a fresh duration,
/// so entering a call cannot reset the caller's remaining wall clock.
#[derive(Debug, Clone, Copy)]
struct CallDeadline(Option<std::time::Instant>);

impl CallDeadline {
    fn for_child(self, own_limit: Option<std::time::Duration>) -> Self {
        let own = own_limit.map(|limit| std::time::Instant::now() + limit);
        Self(match (self.0, own) { (Some(a), Some(b)) => Some(a.min(b)), (a, b) => a.or(b) })
    }
}

#[test]
fn spike_call_ancestry_releases_and_limits_pass_through() {
    use crate::composition::lifecycle::cancellation::{self, CancellationCause};
    let state = RuntimeState::new();
    let caller = std::path::Path::new("/repo/caller.md");
    let helper = std::path::Path::new("/repo/helper.md");
    // Repeated calls: the guard releases the helper's place on return.
    for _ in 0..3 {
        let entered = state.enter_container_proxy(caller, helper, "@helper.md").expect("legal repeated call");
        // While active, the helper cannot call the caller (an active ancestor).
        assert!(state.enter_container_proxy(helper, caller, "@caller.md").is_err());
        drop(entered);
    }
    assert!(state.snapshot().proxy_chain.len() <= 1);
    // A parallel branch clones the snapshot, so its chain is its own.
    let branch = RuntimeState::from_snapshot(state.snapshot());
    let _held = state.enter_container_proxy(caller, helper, "@helper.md").unwrap();
    assert!(branch.enter_container_proxy(caller, helper, "@helper.md").is_ok());
    // Cancellation reaches a child scope entered for the call.
    let parent = cancellation::current();
    let child = parent.child();
    let _scope = child.enter();
    parent.cancel(CancellationCause::Interrupt);
    assert!(cancellation::cause().is_some());
    // Deadline: the child keeps the caller's earlier instant.
    let caller_deadline = CallDeadline(Some(std::time::Instant::now() + std::time::Duration::from_secs(5)));
    let child_deadline = caller_deadline.for_child(Some(std::time::Duration::from_secs(60)));
    assert_eq!(child_deadline.0, caller_deadline.0);
    let tighter = caller_deadline.for_child(Some(std::time::Duration::from_millis(1)));
    assert!(tighter.0 < caller_deadline.0);
}
