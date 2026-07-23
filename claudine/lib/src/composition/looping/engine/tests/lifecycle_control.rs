//! lifecycle control loop-engine tests.

use super::*;
use crate::composition::lifecycle::{
    LifecycleConfig, LifecycleEmitter, LifecycleRuntimeContext, parse_lifecycle_config,
};
use std::sync::Mutex;

/// Records emitted lifecycle signals so tests can assert event ordering and
/// confirm which terminal events were skipped.
#[derive(Default)]
struct SignalRecorder {
    signals: Mutex<Vec<LifecycleSignal>>,
}

impl SignalRecorder {
    fn signals(&self) -> Vec<LifecycleSignal> {
        self.signals.lock().unwrap().clone()
    }
}

impl LifecycleEmitter for SignalRecorder {
    fn emit_stderr(
        &self,
        signal: LifecycleSignal,
        _text: &str,
        _term: &biscuit_terminal::terminal::Terminal,
    ) {
        self.signals.lock().unwrap().push(signal);
    }
    fn emit_message(
        &self,
        _text: &str,
        _source_path: &Path,
        _repo_root: Option<&Path>,
        _messaging: &crate::messaging::RuntimeMessagingSettings,
    ) {
    }
    fn emit_speech(&self, _text: &str, _tts_config: biscuit_speaks::TtsConfig) {}
    fn emit_effect(&self, _name: &str) {}
    fn emit_notification(&self, _title: &str) {}
}

fn lifecycle_from(json: serde_json::Value) -> LifecycleConfig {
    parse_lifecycle_config(&json, Path::new("loop.md")).unwrap()
}

/// Drive `execute_loop_with_lifecycle` against a parsed lifecycle config,
/// counting executor (iteration) invocations. The executor always succeeds
/// so any iteration that runs is unambiguously the engine's decision, not a
/// terminal-signal artifact.
fn run_loop_lifecycle(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    run_loop_lifecycle_with_engine_path(
        prompt_path,
        prompt_path,
        config,
        initial_frontmatter,
        lifecycle,
        emitter,
        invocations,
    )
}

fn run_loop_lifecycle_without_current_ctx(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    run_loop_lifecycle_with_engine_path(
        Path::new(""),
        prompt_path,
        config,
        initial_frontmatter,
        lifecycle,
        emitter,
        invocations,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_loop_lifecycle_with_engine_path(
    engine_prompt_path: &Path,
    source_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_content(
        source_path.parent().unwrap_or(Path::new(".")),
        "",
    );
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path,
        repo_root: source_path.parent(),
        launch_area: None,
        context: Some(&context),
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(source_path.parent().unwrap_or(Path::new(".")))
        .auto_rehash(false)
        .build();
    execute_loop_with_lifecycle(
        engine_prompt_path,
        config,
        initial_frontmatter,
        LoopExecutionOptions::default(),
        lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &crate::composition::lifecycle_executor::SystemShellRunner,
        emitter,
        None,
        |_ctx, _guard| {
            *invocations.borrow_mut() += 1;
            Ok(LoopIterationOutput::success("ran"))
        },
    )
    .unwrap()
}

/// `skip` at `initialize` in a looping document ends the run immediately:
/// zero iterations, no executor invocation, no terminal/`finalize`/`loop`
/// events — the whole-document opt-out the spec requires (spec.md:338).
#[test]
fn loop_initialize_skip_ends_run_with_zero_iterations() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": "skip" }] },
        "finalize": { "stack": [{ "action": {"append_line": ["never.log", "finalize"]} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(result.error.is_none(), "skip is a clean opt-out: {result:?}");
    assert_eq!(result.iteration_count, 0, "no iteration runs after skip");
    assert_eq!(*invocations.borrow(), 0, "the executor must never be invoked");
    assert!(
        result.handoff.is_none(),
        "skip completes the run; it is not a proxy hand-off"
    );
    // Only `initialize` may have emitted (the stack control fires before any
    // terminal handling); no terminal/finalize/loop signal escaped.
    let signals = emitter.signals();
    assert!(
        !signals.contains(&LifecycleSignal::Finalize),
        "skip must not run finalize; got {signals:?}"
    );
    assert!(
        !signals.contains(&LifecycleSignal::Success)
            && !signals.contains(&LifecycleSignal::Failure),
        "skip emits no terminal signal; got {signals:?}"
    );
}

/// `error(...)` at `initialize` routes the run to `failure` then `finalize`
/// and terminates the loop with a typed `LifecycleInitializeFailed` — no
/// iteration runs (initialize fires once, before iterations). Mirrors the
/// non-loop path (spec.md:607).
#[test]
fn loop_initialize_error_routes_to_failure_and_finalize() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": { "stderr": "fail" },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs after an init error");
    assert_eq!(result.iteration_count, 0);
    match &result.error {
        Some(CompositionError::LifecycleInitializeFailed { reason, .. }) => {
            assert!(
                reason.contains("preflight refused"),
                "the authored reason must survive: {reason}"
            );
        }
        other => panic!("expected LifecycleInitializeFailed, got {other:?}"),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Failure),
        "init error routes to failure; got {signals:?}"
    );
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "init error then runs finalize; got {signals:?}"
    );
}

/// `stop` at `initialize` ends only the initialize stack; the run proceeds
/// into the iteration loop unchanged (spec.md:337). Proven by parity: the
/// iteration count with a `stop` init control equals the count from an
/// otherwise-identical document whose initialize stack is benign (an
/// `info(...)` that never re-routes), so `stop` is confirmed to leave the
/// loop untouched without hard-coding the engine's iteration arithmetic.
#[test]
fn loop_initialize_stop_proceeds_into_iterations() {
    let run = |action: serde_json::Value| {
        let config = counter_loop(3);
        let lifecycle = lifecycle_from(json!({
            "initialize": { "stack": [{ "action": action }] },
        }));
        let emitter = SignalRecorder::default();
        let invocations = RefCell::new(0usize);
        let result = run_loop_lifecycle_without_current_ctx(
            Path::new("loop.md"),
            &config,
            object(json!({ "counter": 0 })),
            &lifecycle,
            &emitter,
            &invocations,
        );
        (result, invocations.into_inner())
    };

    let (stop_result, stop_invocations) = run(json!("stop"));
    let (baseline_result, baseline_invocations) = run(json!({ "info": "init ran" }));

    assert!(stop_result.error.is_none(), "stop is benign: {stop_result:?}");
    assert!(stop_invocations > 0, "the loop must run after a benign stop");
    assert_eq!(
        stop_result.iteration_count, baseline_result.iteration_count,
        "stop must not change how many iterations run"
    );
    assert_eq!(
        stop_invocations, baseline_invocations,
        "stop must not change executor invocations vs. a benign init stack"
    );
    assert_eq!(stop_result.iteration_count, stop_invocations);
}

/// `proxy(...)` at `initialize` hands off without running any iteration,
/// terminal, `finalize`, or `loop` event — the caller's coordinator commits
/// the request and re-enters with the target's own `initialize`
/// (spec.md:340,607).
#[test]
fn loop_initialize_proxy_hands_off_without_iterating() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();
    let target = dir.path().join("target.md");
    std::fs::write(&target, "---\n---\nbody").unwrap();

    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"proxy": "target.md"} }] },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(result.error.is_none(), "clean proxy hand-off: {result:?}");
    assert_eq!(result.iteration_count, 0, "no iteration runs on a proxy hand-off");
    assert_eq!(*invocations.borrow(), 0);
    let Some(SurfacedHandoff::Request(request)) = &result.handoff else {
        panic!(
            "the hand-off is surfaced as a request the caller must consume; \
             got {:?}",
            result.handoff
        );
    };
    assert_eq!(
        request.target(),
        "target.md",
        "the authored reference rides on the request; the coordinator resolves it"
    );
    assert_eq!(request.provenance().source_path(), prompt);
    assert!(
        target.exists(),
        "the fixture target exists; the engine neither reads nor resolves it"
    );
    let signals = emitter.signals();
    assert!(
        !signals.contains(&LifecycleSignal::Finalize)
            && !signals.contains(&LifecycleSignal::Failure)
            && !signals.contains(&LifecycleSignal::Success),
        "a clean hand-off fires no terminal/finalize/loop events; got {signals:?}"
    );
}

/// An unresolvable `proxy(...)` target is **not** the engine's failure to
/// report.
///
/// This deliberately reverses an earlier assertion. The engine used to resolve
/// the target itself and route a miss through `failure` + `finalize`, which
/// gave the loop route its own resolution semantics — and, because it could
/// only see a single-element chain, its own cycle semantics too. Resolution now
/// belongs to the coordinator, so an unresolvable target produces the same
/// typed refusal from the same place whether the proxy came from a loop
/// document, a single document, or terminal recovery. What the engine still
/// owes is that no iteration runs and no terminal/finalize event fires.
#[test]
fn loop_initialize_proxy_defers_resolution_to_the_coordinator() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"proxy": "does-not-exist.md"} }] },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs on a proxy hand-off");
    assert!(
        result.error.is_none(),
        "the engine does not consult the filesystem, so it raises nothing here: {:?}",
        result.error
    );
    let Some(SurfacedHandoff::Request(request)) = &result.handoff else {
        panic!("the request is surfaced regardless of whether it resolves");
    };
    assert_eq!(request.target(), "does-not-exist.md");
    let signals = emitter.signals();
    assert!(
        !signals.contains(&LifecycleSignal::Failure)
            && !signals.contains(&LifecycleSignal::Finalize),
        "the source's terminal events belong to the target once proxy is \
         selected; got {signals:?}"
    );
}

// ── Loop-gate `error(...)` tests (Finding 3) ─────────────────────────
//
// The `loop:` gate is a terminal-phase event, so only an *explicit*
// `error(...)` lifecycle action converts the loop's final outcome to
// failure and exits the loop. An *unintentional* action error there must
// leave the outcome unchanged (`routes_to_failure(Loop)` is always false).

/// An explicit `error(...)` in the `loop:` gate stack converts the loop's
/// final outcome to failure and exits — even though the `until` condition
/// would otherwise continue iterating. The error takes precedence over the
/// condition, so the gate's mutation (`increment(counter)`) is NOT applied
/// and no further iteration runs (spec.md:334-341, "convert final outcome
/// to failure and exit the loop").
#[test]
fn loop_gate_explicit_error_fails_and_exits_without_mutation() {
    // `until: counter > 5` with `counter` starting at 0 would continue
    // looping, so an exit here can only come from the gate's `error(...)`.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "action": {"error": "gate rejected final state"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleLoopGateFailed { reason, .. }) => {
            assert!(
                reason.contains("gate rejected final state"),
                "the authored reason must survive: {reason}"
            );
        }
        other => panic!("expected LifecycleLoopGateFailed, got {other:?}"),
    }
    assert_eq!(
        *invocations.borrow(),
        1,
        "exactly one iteration ran before the gate failed the loop"
    );
    assert_eq!(result.iteration_count, 1);
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(0)),
        "the gate mutation must NOT be applied when the gate raises an error"
    );
}

/// An *unintentional* action error in the `loop:` gate stack (a `shell`
/// command that exits non-zero) must NOT invert the outcome: `loop` is a
/// terminal-phase event, so the gate proceeds to the condition and the loop
/// finishes successfully once the condition stops it. Contrast with the
/// explicit-`error` test above.
#[test]
fn loop_gate_unintentional_error_does_not_invert_outcome() {
    // `until: counter > 1` with `counter` starting at 0 and an
    // `increment(counter)` gate mutation: the gate evaluates its condition
    // against the pre-mutation state, so the loop runs two iterations
    // (counter 0→1→2) before the condition stops it. The `shell('false')`
    // gate action errors on every pass but, being unintentional at a
    // terminal-phase event, never aborts the loop.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 1".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "action": {"shell": "false"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert!(
        result.error.is_none(),
        "an unintentional gate action error must not fail the loop: {result:?}"
    );
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(2)),
        "the loop ran to completion: the gate mutation applied on each \
         continuing pass despite the unintentional action error"
    );
}

/// A late-binding **evaluation** error in the `loop:` gate (a crashed
/// `when:` guard) halts the loop *before* the condition is evaluated and
/// before any gate mutation is applied — unlike an unintentional dispatch
/// failure, which is tolerated (Decision #3). The run reports the typed
/// `LifecycleEvaluationError`.
#[test]
fn loop_gate_evaluation_error_fails_before_condition_and_mutation() {
    // `until: counter > 5` with `counter` starting at 0 would loop forever,
    // so an exit here can only come from the gate's evaluation error.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // The gate item's `when:` references an undefined root, so it *raises*
    // at event time rather than evaluating cleanly to false.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(event, "loop", "the failure names the loop gate event");
        }
        other => panic!("expected LifecycleEvaluationError, got {other:?}"),
    }
    assert_eq!(
        *invocations.borrow(),
        1,
        "exactly one iteration ran before the gate evaluation error halted the loop"
    );
    assert_eq!(
        result.final_frontmatter.get("counter"),
        Some(&json!(0)),
        "the gate mutation must NOT be applied when the gate raises an evaluation error"
    );
}

/// An explicit `error(...)` at `initialize` whose catch `failure.when:`
/// guard raises (undefined root) surfaces the FAILURE evaluation error —
/// not the original `LifecycleInitializeFailed`. Proves the broken path
/// that previously discarded the failure outcome now threads it through
/// the lifecycle catch protocol.
#[test]
fn loop_initialize_error_with_failure_raise_surfaces_failure_evaluation_error() {
    let config = counter_loop(3);
    // The `initialize` stack raises an explicit `error(...)`, which routes
    // to `failure`. The `failure.when:` references an undefined root, so it
    // *raises* at event time rather than evaluating cleanly to false.
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": {
            "stderr": "fail",
            "stack": [{ "when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": { "stderr": "final" },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    assert_eq!(*invocations.borrow(), 0, "no iteration runs after init error");
    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "failure",
                "the surfaced error must name the failure event (its `when:` raised)"
            );
        }
        other => panic!(
            "expected LifecycleEvaluationError for failure, got {other:?}"
        ),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Failure),
        "init error routes to failure; got {signals:?}"
    );
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "init error then runs finalize; got {signals:?}"
    );
}

/// Drive `execute_loop_with_lifecycle` with an executor that emits the
/// `Success` terminal signal through the guard before returning, so the
/// post-finalize loop gate's `finalize` can actually fire (it is gated on a
/// recorded terminal emission). The standard `run_loop_lifecycle` helper's
/// executor never emits a terminal signal, so `finalize` at the gate would
/// be a no-op there — this helper is needed to prove the loop-gate
/// evaluation-error → `finalize` catch path.
fn run_loop_lifecycle_emitting_terminal(
    prompt_path: &Path,
    config: &LoopConfig,
    initial_frontmatter: Map<String, Value>,
    lifecycle: &LifecycleConfig,
    emitter: &dyn LifecycleEmitter,
    invocations: &RefCell<usize>,
) -> LoopExecutionResult {
    let settings = crate::events::GlobalSettings::default();
    let messaging = crate::messaging::RuntimeMessagingSettings {
        user: None,
        repo: None,
    };
    let term = biscuit_terminal::terminal::Terminal::default();
    let context = darkmatter::markdown::compose::ComposeContext::capture_for_content(
        prompt_path.parent().unwrap_or(Path::new(".")),
        "",
    );
    let lifecycle_ctx = LifecycleRuntimeContext {
        settings: &settings,
        messaging: &messaging,
        term: &term,
        source_path: prompt_path,
        repo_root: prompt_path.parent(),
        launch_area: None,
        context: Some(&context),
    };
    let effect_engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(prompt_path.parent().unwrap_or(Path::new(".")))
        .auto_rehash(false)
        .build();
    let shell_runner = crate::composition::lifecycle_executor::SystemShellRunner;
    let loop_start = std::time::Instant::now();
    execute_loop_with_lifecycle(
        prompt_path,
        config,
        initial_frontmatter,
        LoopExecutionOptions::default(),
        lifecycle,
        &lifecycle_ctx,
        &effect_engine,
        &shell_runner,
        emitter,
        None,
        |ctx, guard| {
            *invocations.borrow_mut() += 1;
            // Emit the terminal `Success` signal so the loop gate's
            // `finalize` is enabled (it requires a recorded terminal
            // emission). The owned `timing`/`current` outlive the borrowed
            // context within this closure body.
            let (timing, current) = capture_loop_lifecycle_globals(
                prompt_path.parent(),
                lifecycle_ctx.launch_area,
                loop_start,
            );
            let success_ctx = build_loop_stack_context(
                LifecycleSignal::Success,
                &ctx.frontmatter,
                &lifecycle_ctx,
                &effect_engine,
                &shell_runner,
                emitter,
                prompt_path.parent(),
                None,
                Some(&timing),
                Some(&current),
            );
            guard.execute_event(LifecycleSignal::Success, &success_ctx);
            Ok(LoopIterationOutput::success("ran"))
        },
    )
    .unwrap()
}

/// A late-binding **evaluation** error in the `loop:` gate is a
/// terminal-phase raise (Decision #3): it must fire `finalize` exactly once
/// carrying the loop error as the `err` global, so a `finalize.stack` can
/// react. Proven by a `finalize` stack whose `append_line` is gated on
/// `err.variant` — the line only lands if `err` reached `finalize`.
#[test]
fn loop_gate_evaluation_error_fires_finalize_with_err() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    // `until: counter > 5` with `counter` at 0 would loop forever, so the
    // only exit is the gate's evaluation error.
    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // The gate `when:` references an undefined root, so it *raises* at event
    // time. `finalize` (top-level `stderr` fires so the recorder logs the
    // signal) writes the threaded `err` fields to a log, gated on the
    // canonical `when: "err"` truthiness guard.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
        "finalize": {
            "stderr": "done",
            "stack": [{
                "when": "err",
                "action": {"append_line": ["err.log", "{{ err.variant + '|' + err.msg }}"]}
            }]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle_emitting_terminal(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(event, "loop", "the surfaced error names the loop gate");
        }
        other => panic!("expected LifecycleEvaluationError for loop, got {other:?}"),
    }
    let signals = emitter.signals();
    assert!(
        signals.contains(&LifecycleSignal::Finalize),
        "a loop-gate evaluation error must fire finalize; got {signals:?}"
    );
    let log = dir.path().join("err.log");
    let contents = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(
        !contents.trim().is_empty(),
        "finalize must see `err` from the loop evaluation error (err.log empty)"
    );
    assert!(
        contents.contains('|'),
        "finalize received both err.variant and err.msg: {contents:?}"
    );
}

/// When the `loop:` gate raises AND the catch `finalize` itself raises, the
/// surfaced error must name `finalize` (the latest crash) — precedence
/// finalize > loop. A raise inside `finalize` must not re-enter `finalize`.
#[test]
fn loop_gate_evaluation_error_with_finalize_raise_surfaces_finalize() {
    let dir = TempDir::new().unwrap();
    let prompt = dir.path().join("loop.md");
    std::fs::write(&prompt, "---\n---\nbody").unwrap();

    let config = LoopConfig {
        condition: LoopCondition::Until("counter > 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };
    // `finalize` top-level `stderr` fires (recorder logs the signal) before
    // its stack `when:` raises on an undefined root.
    let lifecycle = lifecycle_from(json!({
        "loop": { "stack": [{ "when": "missing_root == true", "action": {"stderr": "x"} }] },
        "finalize": {
            "stderr": "done",
            "stack": [{ "when": "also_missing == true", "action": {"stderr": "never"} }]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle_emitting_terminal(
        &prompt,
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "finalize",
                "the surfaced error must name finalize (latest crash)"
            );
        }
        other => panic!("expected LifecycleEvaluationError for finalize, got {other:?}"),
    }
    let finalize_count = emitter
        .signals()
        .iter()
        .filter(|s| **s == LifecycleSignal::Finalize)
        .count();
    assert_eq!(
        finalize_count, 1,
        "a raise inside finalize must not re-enter finalize"
    );
}

/// When both `failure.when` and `finalize.when` raise after an explicit
/// `initialize.error(...)`, the surfaced error must name `finalize` (the
/// latest lifecycle crash) — not `failure` or `initialize`. This proves
/// the precedence rule (finalize > failure > original) holds for the
/// previously-broken explicit-error catch path.
#[test]
fn loop_initialize_error_with_failure_and_finalize_raise_surfaces_finalize() {
    let config = counter_loop(3);
    let lifecycle = lifecycle_from(json!({
        "initialize": { "stack": [{ "action": {"error": "preflight refused"} }] },
        "failure": {
            "stderr": "fail",
            "stack": [{ "when": "missing_root == true", "action": {"stderr": "never"}}]
        },
        "finalize": {
            "stderr": "final",
            "stack": [{ "when": "also_missing == true", "action": {"stderr": "never"}}]
        },
    }));
    let emitter = SignalRecorder::default();
    let invocations = RefCell::new(0usize);

    let result = run_loop_lifecycle(
        Path::new("loop.md"),
        &config,
        object(json!({ "counter": 0 })),
        &lifecycle,
        &emitter,
        &invocations,
    );

    match &result.error {
        Some(CompositionError::LifecycleEvaluationError { event, .. }) => {
            assert_eq!(
                event, "finalize",
                "the surfaced error must name the finalize event (latest crash)"
            );
        }
        other => panic!(
            "expected LifecycleEvaluationError for finalize, got {other:?}"
        ),
    }
}
