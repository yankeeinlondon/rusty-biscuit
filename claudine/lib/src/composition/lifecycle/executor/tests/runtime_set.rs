//! The `set` side effect: routing into the invocation-local runtime layer.

use super::*;

use crate::composition::RuntimeState;

/// Run `config`'s `signal` event against a fresh live cell + the supplied
/// runtime cell, returning the emitted events.
#[allow(clippy::too_many_arguments)]
fn run_event(
    config: &LifecycleConfig,
    signal: LifecycleSignal,
    base: &Map<String, Value>,
    live: &std::cell::RefCell<Map<String, Value>>,
    runtime: &RuntimeState,
    engine: &EffectEngine,
) -> (LifecycleEventOutcome, Vec<Emitted>) {
    let shell = MockShell::new(0);
    let harness = Harness::default();
    let recorder = Recorder::default();
    let context = ctx_with_runtime(
        signal,
        base,
        live,
        runtime,
        engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(config);
    (outcome, recorder.events())
}

fn config(value: Value) -> LifecycleConfig {
    parse_lifecycle_config(&value, Path::new("t.md")).unwrap()
}

/// The positional form writes the runtime layer and a later action in the same
/// stack reads the new value.
#[test]
fn set_is_visible_to_a_later_action_in_the_same_stack() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"set": ["phase", "build"]},
        {"message": "phase={{phase}}"}
    ]}]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    let (outcome, events) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, vec![Emitted::Message("phase=build".to_string())]);
    assert_eq!(runtime.snapshot().mutations.get("phase"), Some(&json!("build")));
}

/// The key/value action form is equivalent to the positional form.
#[test]
fn the_key_value_form_writes_the_same_runtime_layer() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"action": "set", "key": "phase", "value": "ship"},
        {"message": "phase={{phase}}"}
    ]}]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    let (outcome, events) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );

    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, vec![Emitted::Message("phase=ship".to_string())]);
    assert_eq!(runtime.snapshot().mutations.get("phase"), Some(&json!("ship")));
}

/// A whole-value `{{ }}` span keeps its typed value through `set`, so the
/// stored mutation is a boolean/number/array, not the rendered string.
#[test]
fn a_whole_value_span_keeps_its_type() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"set": ["ready", "{{ true }}"]},
        {"set": ["retries", "{{ 2 + 1 }}"]},
        {"set": ["items", "{{ tags }}"]}
    ]}]}}));
    let base = map(json!({"tags": ["a", "b"]}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    let (outcome, _) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );
    assert_eq!(outcome, LifecycleEventOutcome::default());

    let mutations = runtime.snapshot().mutations;
    assert_eq!(mutations.get("ready"), Some(&json!(true)));
    assert_eq!(mutations.get("retries"), Some(&json!(3)));
    assert_eq!(mutations.get("items"), Some(&json!(["a", "b"])));
}

/// A mutation written by `start` is still visible in `success` *and* survives
/// into the runtime layer for the next composition.
#[test]
fn a_mutation_in_start_is_visible_to_a_later_event() {
    let config = config(json!({
        "start": {"stack": [{"action": {"set": ["phase", "running"]}}]},
        "success": {"message": "phase={{phase}}"}
    }));
    let base = map(json!({"phase": "pending"}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    run_event(
        &config,
        LifecycleSignal::Start,
        &base,
        &live,
        &runtime,
        &engine,
    );
    let (_, events) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );

    assert_eq!(events, vec![Emitted::Message("phase=running".to_string())]);
    assert_eq!(runtime.snapshot().mutations.get("phase"), Some(&json!("running")));
}

/// `set` never touches the filesystem — that is the whole distinction from
/// `set_frontmatter`.
#[test]
fn set_writes_no_file() {
    let config = config(json!({"success": {"stack": [
        {"action": {"set": ["phase", "build"]}}
    ]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (dir, engine) = temp_engine();
    let source = dir.path().join("t.md");
    std::fs::write(&source, "---\nphase: plan\n---\nbody\n").unwrap();

    run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );

    assert_eq!(
        std::fs::read_to_string(&source).unwrap(),
        "---\nphase: plan\n---\nbody\n",
        "the document on disk is untouched"
    );
    assert_eq!(runtime.snapshot().mutations.get("phase"), Some(&json!("build")));
}

/// Every reserved root key is refused as a dispatch failure naming `set`, and
/// the runtime layer is left clean.
#[test]
fn set_refuses_every_reserved_root_key() {
    for key in ["state", "previous", "next", "outputs", "sequence_id"] {
        let config = config(json!({"success": {"stack": [
            {"action": {"set": [key, "hijacked"]}}
        ]}}));
        let base = map(json!({}));
        let live = std::cell::RefCell::new(base.clone());
        let runtime = RuntimeState::new();
        let (_dir, engine) = temp_engine();

        let (outcome, _) = run_event(
            &config,
            LifecycleSignal::Success,
            &base,
            &live,
            &runtime,
            &engine,
        );

        let error = outcome
            .action_error
            .unwrap_or_else(|| panic!("`set: [{key}, …]` must fail the event"));
        assert!(
            error.msg.contains(key) && error.msg.contains("reserved"),
            "{key} produced {:?}",
            error.msg
        );
        assert!(runtime.snapshot().mutations.is_empty());
        assert!(live.borrow().get(key).is_none(), "{key} must not leak into live state");
    }
}

/// A dotted path is refused: v1 `set` writes top-level keys only.
#[test]
fn set_refuses_a_dotted_key() {
    let config = config(json!({"success": {"stack": [
        {"action": {"set": ["a.b", "x"]}}
    ]}}));
    let base = map(json!({}));
    let live = std::cell::RefCell::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    let (outcome, _) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );

    assert!(outcome.action_error.is_some(), "a dotted key must fail the event");
    assert!(runtime.snapshot().mutations.is_empty());
}

/// Without a runtime cell (a single-event library caller) `set` still applies
/// intra-stack and still enforces the reserved-key policy — an author must not
/// get a different answer depending on the caller's wiring.
#[test]
fn without_a_runtime_cell_set_still_applies_and_still_refuses_reserved_keys() {
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let harness = Harness::default();

    let applied = config(json!({"success": {"stack": [{"action": [
        {"set": ["phase", "build"]},
        {"message": "phase={{phase}}"}
    ]}]}}));
    let fm = map(json!({"phase": "plan"}));
    let recorder = Recorder::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    assert_eq!(context.execute_event(&applied), LifecycleEventOutcome::default());
    assert_eq!(
        recorder.events(),
        vec![Emitted::Message("phase=build".to_string())]
    );

    let refused = config(json!({"success": {"stack": [
        {"action": {"set": ["outputs", "hijacked"]}}
    ]}}));
    let recorder = Recorder::default();
    let context = ctx(
        LifecycleSignal::Success,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    assert!(context.execute_event(&refused).action_error.is_some());
}

/// `outputs` is readable through `last(outputs)` at event time and reflects
/// what the executor committed — the temporal view the spec assigns to
/// `success`/`finalize`.
#[test]
fn last_outputs_reads_the_committed_accumulator() {
    let config = config(json!({"success": {"message": "prev={{ last(outputs) }}"}}));
    let runtime = RuntimeState::new();
    runtime.append_output("first run\n");
    runtime.append_output("second run");

    let base = map(json!({}));
    let mut seeded = base.clone();
    seeded.insert("outputs".into(), runtime.outputs_value());
    let live = std::cell::RefCell::new(seeded);
    let (_dir, engine) = temp_engine();

    let (_, events) = run_event(
        &config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );
    assert_eq!(events, vec![Emitted::Message("prev=second run".to_string())]);
}
