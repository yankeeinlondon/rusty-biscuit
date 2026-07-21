//! `proxy.with` event-time evaluation into the typed overlay a handoff carries.

use super::*;

use crate::composition::lifecycle::validate_no_err_in_no_error_events;
use crate::composition::{ActionLocation, EvaluatedProxyRequest, ProxyProvenance};

/// Run the `failure` stack of `fm`'s lifecycle config against `state`, with an
/// `err` global available (the event that most `with:` fixtures use).
fn run_failure(
    fm_config: Value,
    state: Value,
    err: Option<&LifecycleErrorInfo>,
) -> LifecycleEventOutcome {
    let config = parse_lifecycle_config(&fm_config, Path::new("router.md")).expect("config parses");
    let fm = map(state);
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        err,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("router.md"),
    );
    context.execute_event(&config)
}

/// The overlay + target of a stack run that is expected to proxy.
fn proxy_of(outcome: &LifecycleEventOutcome) -> (&str, &IndexMap<String, Value>, ActionLocation) {
    match outcome.control.as_ref() {
        Some(StackControl::Proxy {
            target,
            overlay,
            location,
        }) => (target, overlay, *location),
        other => panic!("expected a proxy handoff, got: {other:?}\n{outcome:?}"),
    }
}

/// A `failure` stack with one proxy action carrying `with`.
fn proxy_stack(with: Value) -> Value {
    json!({
        "failure": {
            "stack": [{"action": {
                "action": "proxy",
                "target": "@next.md",
                "with": with
            }}]
        }
    })
}

// ---------------------------------------------------------------------------
// Value semantics
// ---------------------------------------------------------------------------

#[test]
fn whole_value_span_preserves_bool_rather_than_stringifying() {
    // The headline typing rule, and the one that separates `with:` from
    // `proxy.target`: target routes through `render_message`, which collapses
    // every value to a display string. `with:` must not.
    let outcome = run_failure(proxy_stack(json!({"x": "{{ true }}"})), json!({}), None);
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("x"), Some(&json!(true)));
    assert_ne!(overlay.get("x"), Some(&json!("true")));
}

#[test]
fn whole_value_spans_preserve_every_resolved_type() {
    let outcome = run_failure(
        proxy_stack(json!({
            "flag": "{{ src_bool }}",
            "count": "{{ src_number }}",
            "text": "{{ src_string }}",
            "list": "{{ src_array }}",
            "map": "{{ src_object }}",
            "nothing": "{{ src_null }}"
        })),
        json!({
            "src_bool": false,
            "src_number": 7,
            "src_string": "hello",
            "src_array": [1, "two"],
            "src_object": {"k": "v"},
            "src_null": null
        }),
        None,
    );
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("flag"), Some(&json!(false)));
    assert_eq!(overlay.get("count"), Some(&json!(7)));
    assert_eq!(overlay.get("text"), Some(&json!("hello")));
    assert_eq!(overlay.get("list"), Some(&json!([1, "two"])));
    assert_eq!(overlay.get("map"), Some(&json!({"k": "v"})));
    assert_eq!(overlay.get("nothing"), Some(&json!(null)));
}

#[test]
fn mixed_string_resolves_to_a_string() {
    let outcome = run_failure(
        proxy_stack(json!({"label": "phase-{{ iteration }}"})),
        json!({"iteration": 3}),
        None,
    );
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("label"), Some(&json!("phase-3")));
}

#[test]
fn authored_yaml_scalars_and_nulls_keep_their_types() {
    let outcome = run_failure(
        proxy_stack(json!({
            "ready": true,
            "count": 3,
            "name": "router",
            "cleared": null
        })),
        json!({}),
        None,
    );
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("ready"), Some(&json!(true)));
    assert_eq!(overlay.get("count"), Some(&json!(3)));
    assert_eq!(overlay.get("name"), Some(&json!("router")));
    assert_eq!(overlay.get("cleared"), Some(&json!(null)));
}

#[test]
fn nested_strings_follow_the_same_interpolation_rule() {
    // Arrays and objects are data values; every string inside them is typed by
    // the same whole-value-versus-mixed rule as a top-level value.
    let outcome = run_failure(
        proxy_stack(json!({
            "files": ["a.md", "{{ changed }}", "phase-{{ iteration }}"],
            "metadata": {
                "source": "router",
                "attempt": "{{ iteration }}",
                "enabled": "{{ flag }}",
                "deep": {"label": "run-{{ iteration }}"}
            }
        })),
        json!({"changed": "b.md", "iteration": 2, "flag": true}),
        None,
    );
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("files"), Some(&json!(["a.md", "b.md", "phase-2"])));
    assert_eq!(
        overlay.get("metadata"),
        Some(&json!({
            "source": "router",
            "attempt": 2,
            "enabled": true,
            "deep": {"label": "run-2"}
        }))
    );
}

#[test]
fn empty_and_omitted_with_both_yield_an_empty_overlay() {
    let omitted = run_failure(
        json!({"failure": {"stack": [{"action": {"action": "proxy", "target": "@next.md"}}]}}),
        json!({}),
        None,
    );
    let empty = run_failure(proxy_stack(json!({})), json!({}), None);
    let (_, omitted_overlay, _) = proxy_of(&omitted);
    let (_, empty_overlay, _) = proxy_of(&empty);
    assert!(omitted_overlay.is_empty());
    assert_eq!(omitted_overlay, empty_overlay);
}

// ---------------------------------------------------------------------------
// Source state, globals, and scoping
// ---------------------------------------------------------------------------

#[test]
fn with_sees_a_preceding_set_frontmatter_in_the_same_stack() {
    // `with:` resolves against the *live* document state at the moment the
    // proxy fires, not a snapshot taken when the event started.
    let config = parse_lifecycle_config(
        &json!({
            "failure": {
                "stack": [{"action": [
                    {"set_frontmatter": ["router.md", "phase", "two"]},
                    {"action": "proxy", "target": "@next.md", "with": {"phase": "{{ phase }}"}}
                ]}]
            }
        }),
        Path::new("router.md"),
    )
    .expect("config parses");

    let (dir, engine) = temp_engine();
    std::fs::write(dir.path().join("router.md"), "---\nphase: one\n---\nbody\n").unwrap();

    let fm = map(json!({"phase": "one"}));
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    // A bare file name so the source resolves against the engine's mutation
    // root exactly as the `set_frontmatter` target does.
    let context = ctx(
        LifecycleSignal::Failure,
        &fm,
        None,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("router.md"),
    );

    let outcome = context.execute_event(&config);
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(
        overlay.get("phase"),
        Some(&json!("two")),
        "the mutation made earlier in this stack must be visible to `with:`"
    );
}

#[test]
fn err_global_is_readable_in_a_failure_with() {
    let err = LifecycleErrorInfo::from_action_failure("shell", "boom");
    let outcome = run_failure(
        proxy_stack(json!({"reason": "{{ err.msg }}", "kind": "{{ err.variant }}"})),
        json!({}),
        Some(&err),
    );
    let (_, overlay, _) = proxy_of(&outcome);
    assert_eq!(overlay.get("reason"), Some(&json!("boom")));
    assert_eq!(overlay.get("kind"), Some(&json!("shell")));
}

#[test]
fn out_of_scope_err_in_a_with_value_is_rejected_before_the_event_fires() {
    // `err` is out of scope on an event that carries no error snapshot. Now
    // that `with:` values are expression trees they are reachable from the
    // shared surface walk, so the same static scan that guards `target` and
    // every message body guards them too — at prepare time, not by failing
    // mid-handoff.
    for (event, with) in [
        ("initialize", json!({"reason": "{{ err.msg }}"})),
        ("start", json!({"meta": {"reason": "{{ err.msg }}"}})),
        ("success", json!({"list": ["ok", "{{ err.variant }}"]})),
    ] {
        let config = parse_lifecycle_config(
            &json!({event: {"stack": [{"action": {
                "action": "proxy",
                "target": "@next.md",
                "with": with
            }}]}}),
            Path::new("router.md"),
        )
        .expect("config parses");

        let err = validate_no_err_in_no_error_events(&config, Path::new("router.md"))
            .expect_err("`err` is out of scope on {event}");
        match err {
            CompositionError::LifecycleErrNotAvailable {
                property,
                event: got,
                ..
            } => {
                assert_eq!(got, event);
                assert!(
                    property.starts_with(&format!("{event}.stack[0].action[0].with.")),
                    "diagnostic roots at the offending `with` value: {property}"
                );
            }
            other => panic!("expected LifecycleErrNotAvailable for {event}, got: {other:?}"),
        }
    }
}

#[test]
fn err_in_a_with_value_is_allowed_on_an_error_carrying_event() {
    // The mirror of the scan above: `failure` carries `err`, so the same
    // authoring is legal there.
    let config = parse_lifecycle_config(
        &proxy_stack(json!({"reason": "{{ err.msg }}"})),
        Path::new("router.md"),
    )
    .expect("config parses");
    assert!(validate_no_err_in_no_error_events(&config, Path::new("router.md")).is_ok());
}

#[test]
fn a_raw_span_stored_in_frontmatter_never_reaches_the_overlay() {
    // A frontmatter value that is itself template text would otherwise ride
    // into the overlay and be evaluated a second time, at the target. It must
    // not — including from inside a resolved container.
    for (label, state) in [
        ("scalar", json!({"payload": "{{ target_side }}"})),
        ("nested", json!({"payload": {"inner": "{{ target_side }}"}})),
        ("array", json!({"payload": ["ok", "{{ target_side }}"]})),
    ] {
        let outcome = run_failure(proxy_stack(json!({"x": "{{ payload }}"})), state, None);
        assert!(
            outcome.control.is_none(),
            "{label}: a surviving span must abort the handoff"
        );
        let info = outcome
            .evaluation_error
            .unwrap_or_else(|| panic!("{label}: expected an evaluation error"));
        assert_eq!(info.variant, "LifecycleProxyWithEvaluationFailed", "{label}");
    }
}

// ---------------------------------------------------------------------------
// Failure behavior
// ---------------------------------------------------------------------------

#[test]
fn failure_names_the_exact_nested_path_without_echoing_other_values() {
    let outcome = run_failure(
        proxy_stack(json!({
            "secret": "s3cret-value",
            "metadata": {"area": ["ok", "{{ missing_root }}"]}
        })),
        json!({}),
        None,
    );
    assert!(outcome.control.is_none());
    let info = outcome.evaluation_error.expect("unknown root raises");
    assert_eq!(info.variant, "LifecycleProxyWithEvaluationFailed");
    assert!(
        info.msg.contains("failure.stack[0].action[0].with.metadata.area[1]"),
        "message roots at the exact nested path: {}",
        info.msg
    );
    assert!(
        info.msg.contains("@next.md"),
        "message names the target: {}",
        info.msg
    );
    assert!(
        !info.msg.contains("s3cret-value"),
        "an unrelated overlay value must not be echoed: {}",
        info.msg
    );
}

#[test]
fn malformed_expression_and_unknown_function_raise() {
    for with in [
        // Unknown function, whole-value and mixed.
        json!({"x": "{{ nope(1) }}"}),
        json!({"x": "prefix {{ nope(1) }}"}),
        // Malformed span inside a mixed string: parsing is deferred to DM2, so
        // unlike a whole-value span this only fails at event time.
        json!({"x": "prefix {{ 1 + }}"}),
    ] {
        let outcome = run_failure(proxy_stack(with.clone()), json!({}), None);
        assert!(outcome.control.is_none(), "{with}: no handoff");
        assert_eq!(
            outcome
                .evaluation_error
                .unwrap_or_else(|| panic!("{with}: expected a raise"))
                .variant,
            "LifecycleProxyWithEvaluationFailed",
            "{with}"
        );
    }
}

#[test]
fn evaluation_is_atomic_across_the_whole_mapping() {
    // A later failing key must not leave the earlier keys installed anywhere,
    // and the source stays active for attribution. The overlay only exists on
    // the handoff, so "no partial overlay" is observable as "no handoff".
    let outcome = run_failure(
        proxy_stack(json!({"good": "{{ present }}", "bad": "{{ absent_root }}"})),
        json!({"present": "value"}),
        None,
    );
    assert!(
        outcome.control.is_none(),
        "one failing value aborts the whole handoff"
    );
    assert!(outcome.evaluation_error.is_some());
}

#[test]
fn a_failing_target_never_evaluates_the_overlay() {
    // Target resolution comes first, so an unresolvable target aborts before
    // the mapping is touched — the diagnostic is the target's, not `with`'s.
    let outcome = run_failure(
        json!({
            "failure": {
                "stack": [{"action": {
                    "action": "proxy",
                    "target": "{{ absent_root }}",
                    "with": {"x": "{{ also_absent }}"}
                }}]
            }
        }),
        json!({}),
        None,
    );
    assert!(outcome.control.is_none());
    let info = outcome.evaluation_error.expect("target raise");
    assert_eq!(info.variant, "proxy", "the target failure is attributed to the action");
}

#[test]
fn no_error_does_not_suppress_an_overlay_failure() {
    // `no_error` is universal on key/value actions and stays accepted, but it
    // only ever suppressed *dispatch*. Proxy has no dispatch phase, so an
    // overlay raise remains fatal.
    let outcome = run_failure(
        json!({
            "failure": {
                "stack": [{"action": {
                    "action": "proxy",
                    "target": "@next.md",
                    "no_error": true,
                    "with": {"x": "{{ absent_root }}"}
                }}]
            }
        }),
        json!({}),
        None,
    );
    assert!(outcome.control.is_none(), "`no_error` must not smuggle a handoff through");
    assert!(
        outcome.action_error.is_none(),
        "an overlay raise is an evaluation error, not a suppressible dispatch error"
    );
    assert_eq!(
        outcome
            .evaluation_error
            .expect("still raises under no_error")
            .variant,
        "LifecycleProxyWithEvaluationFailed"
    );
}

// ---------------------------------------------------------------------------
// Handoff assembly
// ---------------------------------------------------------------------------

#[test]
fn location_identifies_the_authored_action() {
    let outcome = run_failure(
        json!({
            "failure": {
                "stack": [
                    {"when": "false", "action": {"say": "skipped"}},
                    {"action": [
                        {"info": "noise"},
                        {"action": "proxy", "target": "@next.md", "with": {"x": 1}}
                    ]}
                ]
            }
        }),
        json!({}),
        None,
    );
    let (_, _, location) = proxy_of(&outcome);
    assert_eq!(location.signal(), LifecycleSignal::Failure);
    assert_eq!(location.stack_index(), 1);
    assert_eq!(location.action_index(), 1);
    assert_eq!(location.to_string(), "failure.stack[1].action[1]");
}

#[test]
fn a_handoff_assembles_into_an_evaluated_request_losslessly() {
    // Evaluation owns everything but the invocation-wide chain, which is run
    // ledger state the coordinator supplies. Nothing else is missing.
    let outcome = run_failure(
        proxy_stack(json!({"attempt": "{{ iteration }}"})),
        json!({"iteration": 4}),
        None,
    );
    let (target, overlay, location) = proxy_of(&outcome);
    let chain = vec![PathBuf::from("router.md")];
    let request = EvaluatedProxyRequest::new(
        target.to_string(),
        overlay.clone(),
        ProxyProvenance::new(PathBuf::from("router.md"), location, chain.clone()),
    );

    assert_eq!(request.target(), "@next.md");
    assert_eq!(request.overlay().get("attempt"), Some(&json!(4)));
    assert_eq!(request.provenance().signal(), LifecycleSignal::Failure);
    assert_eq!(request.provenance().source_path(), Path::new("router.md"));
    assert_eq!(request.provenance().chain(), chain.as_slice());
}

#[test]
fn nested_ctx_refs_resolve_against_one_captured_snapshot() {
    take_proxy_with_fallback_capture_hints();
    let outcome = run_failure(
        proxy_stack(json!({
            "host_os": "{{ ctx.os }}",
            "nested": {"deep": ["{{ ctx.agent }}"]}
        })),
        json!({}),
        None,
    );
    let (_, overlay, _) = proxy_of(&outcome);
    let host_os = overlay
        .get("host_os")
        .and_then(Value::as_str)
        .expect("ctx.os resolves to a string");
    let agent = overlay
        .get("nested")
        .and_then(|nested| nested.pointer("/deep/0"))
        .and_then(Value::as_str)
        .expect("ctx.agent resolves to a string");
    assert!(!host_os.is_empty(), "ctx.os must be populated: {overlay:?}");
    assert!(!agent.is_empty(), "ctx.agent must be populated: {overlay:?}");

    let capture_hints = take_proxy_with_fallback_capture_hints();
    assert_eq!(
        capture_hints.len(),
        1,
        "the whole proxy.with overlay must use one fallback capture"
    );
    let combined_hint = &capture_hints[0];
    assert!(
        combined_hint.split_whitespace().any(|path| path == "ctx.os"),
        "the combined scan must include the top-level OS context group: {combined_hint:?}"
    );
    assert!(
        combined_hint
            .split_whitespace()
            .any(|path| path == "ctx.agent"),
        "the combined scan must include the nested agent context group: {combined_hint:?}"
    );
}
