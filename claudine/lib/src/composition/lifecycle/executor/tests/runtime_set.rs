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
    live: &std::sync::Mutex<Map<String, Value>>,
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

fn config_from_markdown(source: &str) -> LifecycleConfig {
    let markdown = darkmatter::markdown::Markdown::try_from_content(source.to_string()).unwrap();
    let frontmatter = Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    );
    super::super::super::parse::parse_lifecycle_config_with_orders(
        &frontmatter,
        Path::new("t.md"),
        Some(markdown.frontmatter()),
    )
    .unwrap()
}

fn set_action(entries: Vec<(&str, Value)>, no_error: bool) -> LifecycleAction {
    let authored = entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect();
    LifecycleAction {
        kind: LifecycleActionKind::RuntimeSet(RuntimeSet::new(authored).unwrap()),
        no_error,
    }
}

#[test]
fn set_can_read_an_absent_destination_as_null() {
    let config = config(json!({"initialize": {"stack": [{"action": [
        {"set": {"epilog": "{{message_to_agent}}", "message_to_agent": null}},
        {"message": "handoff=[{{epilog}}]"}
    ]}]}}));
    for base in [map(json!({})), map(json!({"message_to_agent": null})),
        map(json!({"message_to_agent": "continue phase 3"}))] {
        let expected = base.get("message_to_agent").cloned().unwrap_or(Value::Null);
        let live = std::sync::Mutex::new(base.clone());
        let runtime = RuntimeState::new();
        let (_dir, engine) = temp_engine();
        let (outcome, events) = run_event(
            &config, LifecycleSignal::Initialize, &base, &live, &runtime, &engine,
        );
        assert_eq!(outcome, LifecycleEventOutcome::default());
        assert_eq!(runtime.snapshot().mutations.get("epilog"), Some(&expected));
        assert_eq!(runtime.snapshot().mutations.get("message_to_agent"), Some(&Value::Null));
        assert_eq!(events, vec![Emitted::Message(format!(
            "handoff=[{}]", expected.as_str().unwrap_or("")
        ))]);
    }
}

/// Consecutive actions observe successful commits, while each mapping still
/// resolves from its own pre-action snapshot.
#[test]
fn consecutive_set_actions_observe_each_others_updates() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"set": {"phase": "build"}},
        {"set": {"observed": "{{phase}}", "phase": "ship"}},
        {"message": "observed={{observed}},phase={{phase}}"}
    ]}]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::sync::Mutex::new(base.clone());
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
    assert_eq!(
        events,
        vec![Emitted::Message("observed=build,phase=ship".to_string())]
    );
    let mutations = runtime.snapshot().mutations;
    assert_eq!(mutations.get("observed"), Some(&json!("build")));
    assert_eq!(mutations.get("phase"), Some(&json!("ship")));
}

#[test]
fn mapping_set_swaps_values_in_either_destination_order() {
    for entries in [
        vec![("left", json!("{{right}}")), ("right", json!("{{left}}"))],
        vec![("right", json!("{{left}}")), ("left", json!("{{right}}"))],
    ] {
        let base = map(json!({"left": "A", "right": "B"}));
        let live = std::sync::Mutex::new(base.clone());
        let runtime = RuntimeState::new();
        let (_dir, engine) = temp_engine();
        let shell = MockShell::new(0);
        let recorder = Recorder::default();
        let harness = Harness::default();
        let context = ctx_with_runtime(
            LifecycleSignal::Success,
            &base,
            &live,
            &runtime,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );

        let prior = context
            .dispatch_task_side_effect(&set_action(entries, false), "tasks[0].side_effect.set")
            .unwrap();

        assert_eq!(prior, json!({"left": "A", "right": "B"}));
        let mutations = runtime.snapshot().mutations;
        assert_eq!(mutations.get("left"), Some(&json!("B")));
        assert_eq!(mutations.get("right"), Some(&json!("A")));
        assert_eq!(*live.lock().unwrap(), map(json!({"left": "B", "right": "A"})));
    }
}

#[test]
fn failed_expression_publishes_no_part_of_the_mapping_with_or_without_runtime() {
    let action = set_action(
        vec![("valid", json!("resolved")), ("bad", json!("{{unknown_root}}"))],
        false,
    );
    let base = map(json!({"stable": "kept"}));
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();

    let live = std::sync::Mutex::new(base.clone());
    let runtime = RuntimeState::new();
    let context = ctx_with_runtime(
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    assert!(context
        .dispatch_task_side_effect(&action, "tasks[0].side_effect.set")
        .is_err());
    assert!(runtime.snapshot().mutations.is_empty());
    assert_eq!(*live.lock().unwrap(), base);

    let live = std::sync::Mutex::new(base.clone());
    let context = ctx_with_live(
        LifecycleSignal::Success,
        &base,
        &live,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    assert!(context
        .dispatch_task_side_effect(&action, "tasks[0].side_effect.set")
        .is_err());
    assert_eq!(*live.lock().unwrap(), base);
}

#[test]
fn event_set_failure_projects_its_source_rooted_nested_path_without_committing() {
    use biscuit_terminal::errors::BlockError;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;

    let source_text = "---\nsuccess:\n    stack:\n        - action:\n            - set:\n                stable: changed\n                metadata:\n                    files:\n                        - \"{{unknown_root}}\"\n---\nbody\n";
    let config = config_from_markdown(source_text);
    let base = map(json!({"stable": "kept"}));
    let live = std::sync::Mutex::new(base.clone());
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

    let info = outcome.evaluation_error.expect("the nested value raises");
    let expected = "success.stack[0].action[0].set.metadata.files[0]";
    assert_eq!(info.property.as_deref(), Some(expected));
    assert_eq!(info.variant, "set");
    assert!(runtime.snapshot().mutations.is_empty());
    assert_eq!(*live.lock().unwrap(), base);

    let snapshot = info.snapshot.as_ref().expect("set failures are typed");
    assert_eq!(snapshot.code, "composition.lifecycle_invalid");
    assert_eq!(snapshot.detail["property"], json!(expected));
    assert!(snapshot.message.contains("t.md"), "{}", snapshot.message);
    let err_value = info.to_value();
    assert_eq!(err_value["detail"]["property"], snapshot.detail["property"]);
    assert_eq!(err_value["code"], json!(snapshot.code));

    let diagnostic = CompositionError::lifecycle_evaluation("success", "t.md", &info)
        .enrich_frontmatter_text(source_text, true);
    assert_eq!(
        crate::diagnostics::Diagnostic::detail(&diagnostic)["property"],
        snapshot.detail["property"],
    );
    let excerpt = diagnostic
        .frontmatter_excerpt()
        .expect("the runtime value path selects a frontmatter excerpt");
    assert_eq!(excerpt.highlight_line(), Some(9));
    let appendix = strip_escape_codes(
        excerpt.render_appendix(&biscuit_terminal::terminal::Terminal::new_optimistic(120)),
    );
    assert!(appendix.contains("{{unknown_root}}"), "{appendix}");
    let rendered = strip_escape_codes(diagnostic.report_block_error_optimistic(Some(200)));
    assert!(rendered.contains(expected), "{rendered}");
    assert!(rendered.contains("t.md"), "{rendered}");
}

/// The task-stack rebasing gives `setup:`/`teardown:` items a `{root}[n]`
/// spelling with no `stack` segment. An event's items must keep the
/// `{signal}.stack[n]` spelling they have always had — on both of the
/// executor's stack-loop property sites, which is why one run asserts the
/// guard *and* the action.
#[test]
fn event_stack_items_keep_their_signal_rooted_stack_spelling() {
    let guard_config = config(json!({"success": {"stack": [
        {"when": "unknown_guard", "action": {"set": {"unreached": "value"}}},
    ]}}));
    let action_config = config(json!({"success": {"stack": [
        {"action": {"set": {"stable": "{{ unknown_value }}"}}},
    ]}}));
    let base = map(json!({"stable": "kept"}));
    let live = std::sync::Mutex::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();

    let (guard_outcome, _) = run_event(
        &guard_config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );
    assert_eq!(
        guard_outcome
            .evaluation_error
            .expect("the guard raises")
            .property
            .as_deref(),
        Some("success.stack[0].when"),
    );

    let (action_outcome, _) = run_event(
        &action_config,
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
    );
    assert_eq!(
        action_outcome
            .evaluation_error
            .expect("the value raises")
            .property
            .as_deref(),
        Some("success.stack[0].action[0].set.stable"),
    );
}

#[test]
fn late_invalid_destination_publishes_no_part_of_the_task_side_effect() {
    for invalid in ["outputs", "a.b"] {
        let action = set_action(
            vec![("valid", json!("resolved")), (invalid, json!("refused"))],
            false,
        );
        let base = map(json!({"stable": "kept"}));
        let live = std::sync::Mutex::new(base.clone());
        let runtime = RuntimeState::new();
        let (_dir, engine) = temp_engine();
        let shell = MockShell::new(0);
        let recorder = Recorder::default();
        let harness = Harness::default();
        let context = ctx_with_runtime(
            LifecycleSignal::Success,
            &base,
            &live,
            &runtime,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );

        assert!(context
            .dispatch_task_side_effect(&action, "tasks[0].side_effect.set")
            .is_err());
        assert!(runtime.snapshot().mutations.is_empty(), "{invalid} published a runtime prefix");
        assert_eq!(*live.lock().unwrap(), base, "{invalid} leaked through outer write-back");

        let live = std::sync::Mutex::new(base.clone());
        let context = ctx_with_live(
            LifecycleSignal::Success,
            &base,
            &live,
            &engine,
            &shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        assert!(context
            .dispatch_task_side_effect(&action, "tasks[0].side_effect.set")
            .is_err());
        assert_eq!(*live.lock().unwrap(), base, "{invalid} changed no-runtime working state");
    }
}

#[test]
fn no_error_suppresses_a_batch_refusal_without_exposing_a_partial_write() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"set": {"stable": "changed", "outputs": "refused"}, "no_error": true},
        {"message": "stable={{stable}}"}
    ]}]}}));
    let base = map(json!({"stable": "kept"}));
    let live = std::sync::Mutex::new(base.clone());
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
    assert_eq!(events, vec![Emitted::Message("stable=kept".to_string())]);
    assert!(runtime.snapshot().mutations.is_empty());
    assert_eq!(*live.lock().unwrap(), base);
}

#[test]
fn mapping_result_reports_all_priors_and_preserves_explicit_runtime_null() {
    let base = map(json!({"left": "A", "nullable": "document fallback"}));
    let live = std::sync::Mutex::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();
    runtime.set(&engine, "nullable", Value::Null, &base).unwrap();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx_with_runtime(
        LifecycleSignal::Success,
        &base,
        &live,
        &runtime,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );

    let prior = context
        .dispatch_task_side_effect(
            &set_action(
                vec![("left", json!("B")), ("absent", json!(1)), ("nullable", json!("now set"))],
                false,
            ),
            "tasks[0].side_effect.set",
        )
        .unwrap();
    assert_eq!(prior, json!({"left": "A", "absent": null, "nullable": null}));
    assert_eq!(
        context
            .dispatch_task_side_effect(
                &set_action(Vec::new(), false),
                "tasks[0].side_effect.set",
            )
            .unwrap(),
        json!({}),
    );
}

/// A one-property mapping writes the runtime layer.
#[test]
fn a_one_property_mapping_writes_the_runtime_layer() {
    let config = config(json!({"success": {"stack": [{"action": [
        {"set": {"phase": "ship"}},
        {"message": "phase={{phase}}"}
    ]}]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::sync::Mutex::new(base.clone());
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
        {"set": {
            "ready": "{{ true }}",
            "retries": "{{ 2 + 1 }}",
            "items": "{{ tags }}"
        }}
    ]}]}}));
    let base = map(json!({"tags": ["a", "b"]}));
    let live = std::sync::Mutex::new(base.clone());
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
        "start": {"stack": [{"action": {"set": {"phase": "running"}}}]},
        "success": {"message": "phase={{phase}}"}
    }));
    let base = map(json!({"phase": "pending"}));
    let live = std::sync::Mutex::new(base.clone());
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
        {"action": {"set": {"phase": "build"}}}
    ]}}));
    let base = map(json!({"phase": "plan"}));
    let live = std::sync::Mutex::new(base.clone());
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
        let mut destinations = Map::new();
        destinations.insert(key.to_string(), json!("hijacked"));
        let config = config(json!({"success": {"stack": [
            {"action": {"set": Value::Object(destinations)}}
        ]}}));
        let base = map(json!({
            "state": {"id": "authored"},
            "previous": {"id": "before"},
            "next": {"id": "after"},
            "outputs": ["kept"],
            "sequence_id": "sequence-1"
        }));
        let live = std::sync::Mutex::new(base.clone());
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
            .unwrap_or_else(|| panic!("`set: {{{key}: …}}` must fail the event"));
        assert!(
            error.msg.contains(key) && error.msg.contains("reserved"),
            "{key} produced {:?}",
            error.msg
        );
        assert!(runtime.snapshot().mutations.is_empty());
        assert_eq!(*live.lock().unwrap(), base, "{key} changed an authored reserved view");
    }
}

/// A dotted path is refused: v1 `set` writes top-level keys only.
#[test]
fn set_refuses_a_dotted_key() {
    let config = config(json!({"success": {"stack": [
        {"action": {"set": {"a.b": "x"}}}
    ]}}));
    let base = map(json!({}));
    let live = std::sync::Mutex::new(base.clone());
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
        {"set": {"phase": "build"}},
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
        {"action": {"set": {"outputs": "hijacked"}}}
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
    let live = std::sync::Mutex::new(seeded);
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
