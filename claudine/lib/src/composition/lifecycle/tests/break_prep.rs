//! The `break` and `prep` directives: every accepted and rejected shape,
//! placement (`break` ends its list, `prep` returns), event restrictions, the
//! loop's loose verbs and `gate:`, gate strictness, and the run-time refusal
//! that stands until loop control and the target runner exist.

use super::*;
use actions::LifecycleNode;
use darkmatter::markdown::compose::expression::Expr;

fn parse(frontmatter: serde_json::Value) -> Result<LifecycleConfig, CompositionError> {
    parse_lifecycle_config(&frontmatter, dummy_path())
}

fn parse_ok(frontmatter: serde_json::Value) -> LifecycleConfig {
    parse(frontmatter.clone()).unwrap_or_else(|error| panic!("{frontmatter} parses: {error:?}"))
}

fn parse_err(frontmatter: serde_json::Value) -> CompositionError {
    match parse(frontmatter.clone()) {
        Ok(config) => panic!("{frontmatter} is rejected, got {config:?}"),
        Err(error) => error,
    }
}

/// The directive of a one-item `start` stack.
fn start_directive(item: serde_json::Value) -> LifecycleControlAction {
    control_of(parse_ok(json!({ "start": [item] })).stack(LifecycleSignal::Start))
}

fn control_of(nodes: &[LifecycleNode]) -> LifecycleControlAction {
    match nodes {
        [LifecycleNode::Action(LifecycleAction { kind: LifecycleActionKind::LifecycleControl(control), .. })] => {
            control.clone()
        }
        other => panic!("expected one directive, got {other:?}"),
    }
}

/// The authored property a rejected one-item `start` stack names. Every
/// rejection is a typed, located parse error.
fn rejected_property(item: serde_json::Value) -> String {
    match parse_err(json!({ "start": [item.clone()] })) {
        CompositionError::LifecycleInvalidArgs { property, .. }
        | CompositionError::LifecycleActionInvalidLongForm { property, .. }
        | CompositionError::LifecycleWrongArity { property, .. }
        | CompositionError::LifecycleObjectDataThroughInterpolationParameter { property, .. }
        | CompositionError::LifecycleObjectDataThroughInterpolationPositional { property, .. }
        | CompositionError::LifecycleWithNotMapping { property, .. }
        | CompositionError::LifecycleWithWholeMapping { property, .. } => property,
        other => panic!("{item}: unexpected error {other:?}"),
    }
}

fn reason_text(control: &LifecycleControlAction) -> Option<String> {
    match control {
        LifecycleControlAction::Break { reason, .. } => reason.as_ref().map(|reason| match reason {
            Expr::StringLiteral(text) => text.clone(),
            other => panic!("expected a literal reason, got {other:?}"),
        }),
        other => panic!("expected break, got {other:?}"),
    }
}

fn break_code(control: &LifecycleControlAction) -> u8 {
    match control {
        LifecycleControlAction::Break { code, .. } => *code,
        other => panic!("expected break, got {other:?}"),
    }
}

// ── `break` shapes ─────────────────────────────────────────────────────────

#[test]
fn break_without_an_argument_has_no_reason_and_code_zero() {
    for item in [json!("break"), json!({"break": null}), json!({"break": []}), json!({"break": {}})] {
        let control = start_directive(item.clone());
        assert_eq!(reason_text(&control), None, "{item}");
        assert_eq!(break_code(&control), 0, "{item}");
    }
}

#[test]
fn break_positional_reason_keeps_an_explicit_empty_string() {
    assert_eq!(reason_text(&start_directive(json!({"break": "recurring finding"}))).as_deref(), Some("recurring finding"));
    assert_eq!(reason_text(&start_directive(json!({"break": ""}))).as_deref(), Some(""));
    assert_eq!(reason_text(&start_directive(json!({"break": {"reason": ""}}))).as_deref(), Some(""));
}

#[test]
fn break_long_form_accepts_codes_zero_through_255() {
    for code in [0, 2, 255] {
        let control = start_directive(json!({"break": {"reason": "done", "code": code}}));
        assert_eq!(break_code(&control), code as u8);
        assert_eq!(reason_text(&control).as_deref(), Some("done"));
    }
    assert_eq!(break_code(&start_directive(json!({"break": {"code": 7}}))), 7);
}

#[test]
fn break_code_outside_an_integer_from_0_to_255_is_rejected() {
    for code in [json!(256), json!(-1), json!(1.5), json!("2"), json!("{{ 2 }}"), json!(null), json!([1]), json!({"n": 1})] {
        let item = json!({"break": {"code": code.clone()}});
        assert_eq!(rejected_property(item), "start[0].break", "code {code}");
    }
}

#[test]
fn break_reason_must_be_text() {
    for item in [
        json!({"break": {"reason": ["a"]}}),
        json!({"break": {"reason": {"a": 1}}}),
        json!({"break": {"reason": null}}),
        json!({"break": {"reason": 5}}),
        json!({"break": 5}),
        json!({"break": true}),
        json!({"break": ["a"]}),
        json!({"break": ["a", "b"]}),
    ] {
        assert_eq!(rejected_property(item), "start[0].break");
    }
}

#[test]
fn break_unknown_argument_is_rejected() {
    let error = parse_err(json!({"start": [{"break": {"reason": "x", "exit": 2}}]}));
    assert!(error.to_string().contains("exit"), "{error}");
    assert_eq!(rejected_property(json!({"break": {"reason": "x", "exit": 2}})), "start[0].break");
}

#[test]
fn break_with_overlay_is_rejected() {
    assert!(matches!(
        parse_err(json!({"start": [{"break": {"with": {"a": 1}}}]})),
        CompositionError::LifecycleWithUnsupported { .. }
    ));
}

// ── `prep` shapes ──────────────────────────────────────────────────────────

fn prep_parts(control: &LifecycleControlAction) -> (String, usize) {
    match control {
        LifecycleControlAction::Prep { target: Expr::StringLiteral(target), with } => (target.clone(), with.len()),
        other => panic!("expected a literal prep, got {other:?}"),
    }
}

#[test]
fn prep_positional_and_long_form_carry_target_and_overlay() {
    assert_eq!(prep_parts(&start_directive(json!({"prep": "@helper.md"}))), ("@helper.md".to_string(), 0));
    assert_eq!(prep_parts(&start_directive(json!({"prep": {"target": "@helper.md"}}))), ("@helper.md".to_string(), 0));
    assert_eq!(prep_parts(&start_directive(json!({"prep": {"target": "@helper.md", "with": {}}}))), ("@helper.md".to_string(), 0));
    assert_eq!(
        prep_parts(&start_directive(json!({"prep": {"target": "@helper.md", "with": {"stage": "all", "n": 2}}}))),
        ("@helper.md".to_string(), 2)
    );
    let interpolated = start_directive(json!({"prep": "{{ helper }}"}));
    assert!(matches!(interpolated, LifecycleControlAction::Prep { target: Expr::Variable(_), .. }), "{interpolated:?}");
}

#[test]
fn prep_requires_a_non_empty_string_target() {
    for item in [
        json!("prep"),
        json!({"prep": null}),
        json!({"prep": []}),
        json!({"prep": ""}),
        json!({"prep": "  "}),
        json!({"prep": 5}),
        json!({"prep": ["@a.md"]}),
        json!({"prep": {}}),
        json!({"prep": {"with": {"a": 1}}}),
        json!({"prep": {"target": ""}}),
        json!({"prep": {"target": null}}),
        json!({"prep": {"target": true}}),
        json!({"prep": {"target": ["@a.md"]}}),
    ] {
        let property = match parse_err(json!({ "start": [item.clone()] })) {
            CompositionError::LifecycleInvalidArgs { property, .. }
            | CompositionError::LifecycleActionInvalidLongForm { property, .. }
            | CompositionError::LifecycleWrongArity { property, .. } => property,
            other => panic!("{item}: unexpected error {other:?}"),
        };
        assert_eq!(property, "start[0].prep", "{item}");
    }
}

#[test]
fn prep_with_follows_proxy_with_rules() {
    for with in [json!(null), json!("stage"), json!(["a"]), json!(3)] {
        assert!(
            matches!(
                parse_err(json!({"start": [{"prep": {"target": "@a.md", "with": with.clone()}}]})),
                CompositionError::LifecycleWithNotMapping { .. }
            ),
            "with {with}"
        );
    }
    assert!(matches!(
        parse_err(json!({"start": [{"prep": {"target": "@a.md", "with": "{{ overlay }}"}}]})),
        CompositionError::LifecycleWithWholeMapping { .. }
    ));
    assert!(matches!(
        parse_err(json!({"start": [{"prep": {"target": "@a.md", "with": {"{{ key }}": 1}}}]})),
        CompositionError::LifecycleWithDynamicKey { .. }
    ));
}

#[test]
fn prep_unknown_argument_is_rejected() {
    let error = parse_err(json!({"start": [{"prep": {"target": "@a.md", "args": 1}}]}));
    assert!(matches!(error, CompositionError::LifecycleActionInvalidLongForm { .. }), "{error:?}");
    assert!(error.to_string().contains("args"), "{error}");
}

#[test]
fn directive_operands_have_recorded_authored_sources() {
    // The nested-span lint reads every operand from the source map, and a
    // missing record is an internal error, so each form must be recorded.
    for item in [
        json!({"break": "{{ fm.reason }}"}),
        json!({"break": {"reason": "{{ fm.reason }}", "code": 3}}),
        json!({"prep": "{{ fm.helper }}"}),
        json!({"prep": {"target": "@a.md", "with": {"stage": "{{ fm.stage }}"}}}),
    ] {
        let frontmatter = json!({ "start": [item.clone()] });
        let config = parse_ok(frontmatter.clone());
        validate_no_nested_spans_in_literals(&frontmatter, &config, dummy_path())
            .unwrap_or_else(|error| panic!("{item}: {error:?}"));
    }
}

// ── placement and bundles ──────────────────────────────────────────────────

#[test]
fn an_unconditional_break_must_be_last_in_its_list() {
    for stack in [json!(["break", {"info": "after"}]), json!([{"break": "done"}, {"info": "after"}])] {
        assert!(
            matches!(parse_err(json!({"start": stack.clone()})), CompositionError::LifecycleActionOrder { ref property, .. } if property == "start[0].break"),
            "{stack}"
        );
    }
    // Inside a `then` body the rule is the body's.
    assert!(matches!(
        parse_err(json!({"start": [{"when": "true", "then": ["break", {"info": "after"}]}]})),
        CompositionError::LifecycleActionOrder { ref property, .. } if property == "start[0].then[0].break"
    ));
    // A guarded break may precede other items.
    parse_ok(json!({"start": [{"when": "done", "then": ["break"]}, {"info": "after"}]}));
}

#[test]
fn an_unconditional_prep_may_precede_other_items() {
    let config = parse_ok(json!({"start": [
        {"prep": "@helper.md"},
        {"info": "after the call"},
        {"when": "true", "then": [{"prep": "@other.md"}, {"info": "after the branch call"}]},
        "stop"
    ]}));
    assert_eq!(config.stack(LifecycleSignal::Start).len(), 4);
}

#[test]
fn a_bundle_holds_one_directive_and_runs_it_after_its_other_verbs() {
    let config = parse_ok(json!({"start": [{"prep": "@helper.md", "info": "first"}]}));
    let paths: Vec<String> = config
        .stack(LifecycleSignal::Start)
        .iter()
        .map(|node| match node {
            LifecycleNode::Action(action) => action.path.to_string(),
            other => panic!("expected actions, got {other:?}"),
        })
        .collect();
    assert_eq!(paths, ["start[0].info", "start[0].prep"]);
    for bundle in [json!({"prep": "@a.md", "stop": null}), json!({"break": null, "prep": "@a.md"}), json!({"break": null, "error": "x"})] {
        assert!(
            matches!(parse_err(json!({"start": [bundle.clone()]})), CompositionError::LifecycleMultipleLifecycleActions { .. }),
            "{bundle}"
        );
    }
}

// ── events ─────────────────────────────────────────────────────────────────

#[test]
fn break_and_prep_are_placement_errors_under_cancelled() {
    for item in [json!("break"), json!({"break": "x"}), json!({"prep": "@a.md"}), json!({"prep": {"target": "@a.md"}})] {
        match parse_err(json!({"cancelled": [item.clone()]})) {
            CompositionError::LifecycleActionPlacement { event, property, .. } => {
                assert_eq!(event, "cancelled", "{item}");
                assert!(property.starts_with("cancelled[0]."), "{item}: {property}");
            }
            other => panic!("{item}: expected a placement error, got {other:?}"),
        }
    }
}

#[test]
fn break_and_prep_are_accepted_in_every_other_event() {
    for event in ["initialize", "start", "success", "blocked", "failure", "finalize"] {
        parse_ok(json!({ event: [{"prep": "@helper.md"}, {"break": {"reason": "done", "code": 1}}] }));
    }
}

#[test]
fn prep_in_initialize_is_not_a_shell_and_real_shells_are_still_refused() {
    let config = parse_ok(json!({"initialize": [
        {"prep": "@stage-all.md"},
        {"when": "true", "then": [{"prep": {"target": "@b.md", "with": {"k": "v"}}}]},
        "break"
    ]}));
    assert_eq!(config.stack(LifecycleSignal::Initialize).len(), 3);
    match parse_err(json!({"initialize": [{"prep": "@stage-all.md"}, {"shell": "git add -A"}]})) {
        CompositionError::LifecycleActionPlacement { action, event, property, .. } => {
            assert_eq!((action.as_str(), event.as_str(), property.as_str()), ("shell", "initialize", "initialize[1].shell"));
        }
        other => panic!("expected the initialize shell refusal, got {other:?}"),
    }
}

#[test]
fn skip_stays_initialize_only() {
    parse_ok(json!({"initialize": ["skip"]}));
    assert!(matches!(parse_err(json!({"start": ["skip"]})), CompositionError::LifecycleActionPlacement { .. }));
}

// ── the loop's lifecycle concerns ──────────────────────────────────────────

#[test]
fn break_and_prep_are_loose_loop_verbs() {
    let config = parse_ok(json!({"loop": {"until": "done", "break": "finished"}}));
    assert_eq!(reason_text(&control_of(config.stack(LifecycleSignal::Loop))).as_deref(), Some("finished"));
    let config = parse_ok(json!({"loop": {"until": "done", "prep": "@refresh.md"}}));
    assert_eq!(prep_parts(&control_of(config.stack(LifecycleSignal::Loop))).0, "@refresh.md");
    assert!(matches!(
        parse_err(json!({"loop": {"until": "done", "prep": ""}})),
        CompositionError::LifecycleInvalidArgs { ref property, .. } if property == "loop.prep"
    ));
}

#[test]
fn break_and_prep_are_accepted_in_a_loop_gate() {
    let config = parse_ok(json!({"loop": {"until": "done", "gate": [{"prep": "@refresh.md"}, {"info": "x"}, {"break": {"code": 3}}]}}));
    assert_eq!(config.stack(LifecycleSignal::Loop).len(), 3);
}

#[test]
fn loop_gate_null_and_empty_dictionary_are_errors() {
    for gate in [json!(null), json!({})] {
        match parse_err(json!({"loop": {"until": "done", "gate": gate.clone()}})) {
            CompositionError::LifecycleStackInvalidShape { property, message, .. } => {
                assert_eq!(property, "loop.gate", "{gate}");
                assert!(message.contains("gate: []"), "{gate}: {message}");
            }
            other => panic!("gate {gate}: expected a shape error, got {other:?}"),
        }
    }
}

#[test]
fn loop_gate_empty_list_is_an_explicit_no_op() {
    assert!(parse_ok(json!({"loop": {"until": "done", "gate": []}})).stack(LifecycleSignal::Loop).is_empty());
}

#[test]
fn loose_verbs_beside_a_gate_stay_an_error() {
    for gate in [json!([]), json!(["stop"])] {
        assert!(matches!(
            parse_err(json!({"loop": {"until": "done", "gate": gate, "prep": "@a.md"}})),
            CompositionError::LifecycleStackInvalidShape { ref property, .. } if property == "loop.prep"
        ));
    }
}

#[test]
fn event_stacks_keep_null_and_empty_dictionary_as_empty() {
    for value in [json!(null), json!({}), json!([])] {
        assert!(parse_ok(json!({"start": value})).stack(LifecycleSignal::Start).is_empty());
    }
}

// ── run time ───────────────────────────────────────────────────────────────

/// Run the `start` stack of `frontmatter` through the executor.
fn run_start(
    frontmatter: serde_json::Value,
    emitter: &RecordingEmitter,
    loop_values: Option<&crate::composition::LoopAmbient>,
) -> executor::LifecycleEventOutcome {
    let config = parse_ok(frontmatter);
    let (settings, messaging, term) = test_ctx();
    let engine_root = tempfile::tempdir().unwrap();
    let engine = darkmatter::effects::EffectEngine::builder()
        .mutation_root(engine_root.path())
        .auto_rehash(false)
        .build();
    let fm = darkmatter::markdown::compose::SharedFrontmatter::default();
    let context = executor::StackExecutionContext {
        seq: None,
        signal: LifecycleSignal::Start,
        scope: None,
        frontmatter: &fm,
        live_frontmatter: None,
        runtime_state: None,
        err: None,
        timing: None,
        current: None,
        group: None,
        loop_values,
        base_dir: None,
        ctx_base_dir: None,
        prepared_context: None,
        file_resolution_context: crate::test_support::process_context(),
        effect_engine: &engine,
        shell_runner: &executor::DisabledShellRunner,
        emitter,
        term: &term,
        source_path: dummy_path(),
        repo_root: None,
        messaging: &messaging,
        settings: &settings,
    };
    context.execute_event(&config)
}

#[test]
fn prep_without_a_target_runner_is_refused_and_never_ignored() {
    for item in [json!({"prep": "@helper.md"}), json!({"prep": {"target": "@helper.md", "no_error": true}})] {
        let emitter = RecordingEmitter::new();
        let stack = json!([{"stderr": "before"}, {"when": "true", "then": [item.clone()]}, {"stderr": "after"}]);
        let outcome = run_start(json!({ "start": stack }), &emitter, None);
        assert_eq!(outcome.control, None, "{item}: a returning call is not a control");
        let error = outcome.evaluation_error.unwrap_or_else(|| panic!("{item}: refused at run time"));
        assert_eq!(error.property.as_deref(), Some("start[1].then[0].prep"), "{item}");
        assert!(error.msg.contains("this caller runs no `prep` targets"), "{item}: {}", error.msg);
        assert_eq!(
            emitter.actions(),
            vec![EmittedAction::Stderr { signal: LifecycleSignal::Start, text: "before".to_string() }],
            "{item}: the refusal halts the event"
        );
    }
}

/// A target runner that records each call and fails the targets named in
/// `failing`.
struct FakeTargets {
    calls: std::sync::Mutex<Vec<(String, serde_json::Value)>>,
    failing: Vec<&'static str>,
}

impl crate::composition::lifecycle::prep::PrepRunner for FakeTargets {
    fn run(&self, call: &crate::composition::lifecycle::prep::PrepCall<'_>) -> Result<(), crate::composition::lifecycle::prep::PrepFailure> {
        let overlay = serde_json::to_value(call.overlay).unwrap();
        self.calls.lock().unwrap().push((call.target.to_string(), overlay));
        if self.failing.contains(&call.target) {
            return Err(crate::composition::lifecycle::prep::PrepFailure { message: "exited with code 3".into(), snapshot: None });
        }
        Ok(())
    }
}

fn with_targets(failing: Vec<&'static str>) -> (std::sync::Arc<FakeTargets>, crate::composition::lifecycle::prep::PrepScope) {
    let targets = std::sync::Arc::new(FakeTargets { calls: Default::default(), failing });
    let scope = crate::composition::lifecycle::prep::install(targets.clone());
    (targets, scope)
}

#[test]
fn prep_returns_to_the_next_item_including_after_its_branch() {
    let (targets, _scope) = with_targets(Vec::new());
    let emitter = RecordingEmitter::new();
    let stack = json!([
        {"prep": {"target": "@first.md", "with": {"stage": "all"}}},
        {"stderr": "between"},
        {"when": "true", "then": [{"prep": "@second.md"}]},
        {"stderr": "after the branch"},
    ]);
    let outcome = run_start(json!({ "start": stack }), &emitter, None);
    assert_eq!(outcome.evaluation_error, None);
    assert_eq!(outcome.action_error, None);
    assert_eq!(outcome.control, None);
    assert_eq!(
        *targets.calls.lock().unwrap(),
        [("@first.md".to_string(), json!({"stage": "all"})), ("@second.md".to_string(), json!({}))]
    );
    assert_eq!(
        emitter.actions(),
        vec![
            EmittedAction::Stderr { signal: LifecycleSignal::Start, text: "between".to_string() },
            EmittedAction::Stderr { signal: LifecycleSignal::Start, text: "after the branch".to_string() },
        ]
    );
}

#[test]
fn a_failed_target_fails_the_event_with_its_cause_unless_no_error_tolerates_it() {
    let (_targets, _scope) = with_targets(vec!["@broken.md"]);
    let emitter = RecordingEmitter::new();
    let outcome = run_start(json!({ "start": [{"prep": "@broken.md"}, {"stderr": "never"}] }), &emitter, None);
    let error = outcome.action_error.expect("a failed call is the event's action error");
    assert!(error.msg.contains("`prep` of `@broken.md` failed: exited with code 3"), "{}", error.msg);
    assert!(emitter.actions().is_empty(), "the event ends at the failed call");

    let emitter = RecordingEmitter::new();
    let outcome = run_start(
        json!({ "start": [{"prep": {"target": "@broken.md", "no_error": true}}, {"stderr": "tolerated"}] }),
        &emitter,
        None,
    );
    assert_eq!(outcome.action_error, None);
    assert_eq!(emitter.actions(), vec![EmittedAction::Stderr { signal: LifecycleSignal::Start, text: "tolerated".to_string() }]);
}

#[test]
fn a_break_outside_any_loop_is_a_located_error_that_no_error_cannot_suppress() {
    for item in [json!("break"), json!({"break": {"reason": "done", "code": 2, "no_error": true}})] {
        let emitter = RecordingEmitter::new();
        let stack = json!([{"stderr": "before"}, {"when": "true", "then": [item.clone()]}]);
        let outcome = run_start(json!({ "start": stack }), &emitter, None);
        assert_eq!(outcome.control, None, "{item}: an unowned break is not a control");
        let error = outcome.evaluation_error.unwrap_or_else(|| panic!("{item}: an error when it fires"));
        assert_eq!(error.property.as_deref(), Some("start[1].then[0].break"), "{item}");
        assert_eq!(error.reason, crate::composition::error::LifecycleEvaluationReason::BreakWithoutLoop, "{item}");
        assert!(error.msg.contains("fired outside any loop"), "{item}: {}", error.msg);
        assert_eq!(emitter.actions().len(), 1, "{item}: the error halts the event");
    }
}

#[test]
fn a_break_records_its_reason_and_code_in_the_nearest_loop_and_ends_the_event() {
    let emitter = RecordingEmitter::new();
    let ambient = crate::composition::LoopAmbient::new(2, false, false, "", 0);
    let stack = json!([{"stderr": "before"}, {"when": "true", "then": [{"break": {"reason": "pass {{ loop.count }}", "code": 7}}]}, {"stderr": "after"}]);
    let outcome = run_start(json!({ "start": stack }), &emitter, Some(&ambient));
    assert_eq!(outcome.evaluation_error, None);
    assert_eq!(
        outcome.control,
        Some(executor::StackControl::Break { reason: Some("pass 2".to_string()), code: 7 })
    );
    assert_eq!(
        ambient.breaks.pending(),
        Some(crate::composition::looping::LoopBreak { reason: Some("pass 2".to_string()), code: 7 })
    );
    assert_eq!(emitter.actions().len(), 1, "the items after the break do not run");
    // A copy of the values with its own slot does not see the break.
    assert!(!ambient.with_isolated_breaks().breaks.is_pending());
}
