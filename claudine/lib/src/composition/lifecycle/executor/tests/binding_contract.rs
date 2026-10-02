//! Event-time lifecycle evaluation under the binding contract: an absent
//! document property is `null`, an unavailable lifecycle global is a typed
//! failure that never reads the document, and a genuine expression defect
//! halts before any side effect. The typed Darkmatter cause survives every
//! wrapper a library caller can see.

use super::*;

use crate::composition::RuntimeState;
use crate::composition::lifecycle::bindings::{EVENT_HAS_NO_ERROR, OUTSIDE_GROUP};
use darkmatter::markdown::compose::expression::{BindingError, ExpressionError};

/// Run `signal`'s event of `lifecycle` over `frontmatter` with a live cell
/// and a runtime layer, returning the outcome, the emissions, and the final
/// live document.
fn run(
    lifecycle: Value,
    signal: LifecycleSignal,
    frontmatter: Value,
) -> (LifecycleEventOutcome, Vec<Emitted>, Map<String, Value>, RuntimeState) {
    let config = parse_lifecycle_config(&lifecycle, Path::new("t.md")).unwrap();
    let base = map(frontmatter);
    let live = std::sync::Mutex::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx_with_runtime(
        signal,
        &base,
        &live,
        &runtime,
        &engine,
        &shell,
        &recorder,
        &harness,
        Path::new("t.md"),
    );
    let outcome = context.execute_event(&config);
    let document = live.into_inner().unwrap();
    (outcome, recorder.events(), document, runtime)
}

/// The unavailable-binding read behind an evaluation failure.
fn unavailable_read(outcome: &LifecycleEventOutcome) -> (String, String) {
    let info = outcome.evaluation_error.as_ref().expect("an evaluation error");
    let cause = info.cause.as_ref().expect("the typed cause is kept");
    match cause.expression_error() {
        Some(ExpressionError::Binding(binding)) => match binding.as_ref() {
            BindingError::Unavailable(read) => (read.root.clone(), read.reason.code().to_string()),
            other => panic!("expected an unavailable read, got {other:?}"),
        },
        other => panic!("expected a binding error, got {other:?}"),
    }
}

#[test]
fn an_absent_property_in_when_skips_the_action_without_an_error() {
    // The reported shape: a guard over an optional input nobody supplied.
    for when in ["review", "spec_fil", "review == 'x'", "!plan"] {
        let (outcome, events, ..) = run(
            json!({"success": {"stack": [
                {"when": when, "action": {"message": "guarded"}},
                {"action": {"message": "after"}}
            ]}}),
            LifecycleSignal::Success,
            json!({"spec_file": "x"}),
        );
        assert!(outcome.evaluation_error.is_none(), "`when: {when}`: {:?}", outcome.evaluation_error);
        let expected = if when == "!plan" {
            vec![Emitted::Message("guarded".into()), Emitted::Message("after".into())]
        } else {
            vec![Emitted::Message("after".into())]
        };
        assert_eq!(events, expected, "`when: {when}`");
    }
}

#[test]
fn an_absent_property_in_a_message_renders_through_null_semantics() {
    let (outcome, events, ..) = run(
        json!({"success": {
            "message": "spec: {{ spec }}{{ plan ? ' plan: ' + plan : '' }}",
            "stack": [
                {"action": {"message": "[{{ review }}]"}},
                {"action": {"message": "{{ review }}"}},
                {"action": {"message": "{{ plan || 'no plan' }}"}}
            ]
        }}),
        LifecycleSignal::Success,
        json!({"spec": "a.md"}),
    );
    assert!(outcome.evaluation_error.is_none(), "{:?}", outcome.evaluation_error);
    assert_eq!(
        events,
        vec![
            Emitted::Message("spec: a.md".into()),
            Emitted::Message("[]".into()),
            Emitted::Message(String::new()),
            Emitted::Message("no plan".into()),
        ]
    );
}

#[test]
fn an_absent_property_in_a_typed_value_resolves_to_null() {
    let (outcome, _, document, runtime) = run(
        json!({"start": {"stack": [
            {"action": {"set": {"copied": "{{ missing }}", "nested": "{{ missing.deep }}"}}},
            {"action": {"action": "proxy", "target": "next.md", "with": {"carry": "{{ missing }}"}}}
        ]}}),
        LifecycleSignal::Start,
        json!({}),
    );
    assert!(outcome.evaluation_error.is_none(), "{:?}", outcome.evaluation_error);
    assert_eq!(document.get("copied"), Some(&Value::Null));
    assert_eq!(document.get("nested"), Some(&Value::Null));
    let mutations = runtime.snapshot().mutations;
    assert_eq!(mutations.get("copied"), Some(&Value::Null), "null is written, not skipped");
    let Some(StackControl::Proxy { overlay, .. }) = outcome.control else {
        panic!("expected the proxy handoff, got {:?}", outcome.control);
    };
    assert_eq!(overlay.get("carry"), Some(&Value::Null));
}

#[test]
fn set_and_proxy_with_stay_atomic_when_a_null_member_precedes_a_failure() {
    let (outcome, events, document, runtime) = run(
        json!({"start": {"stack": [
            {"action": {"set": {"first": "{{ missing }}", "last": "{{ no_such_fn() }}"}}},
            {"action": {"message": "after"}}
        ]}}),
        LifecycleSignal::Start,
        json!({"first": "kept"}),
    );
    assert!(outcome.evaluation_error.is_some());
    assert_eq!(document.get("first"), Some(&json!("kept")), "nothing is published");
    assert!(runtime.snapshot().mutations.is_empty());
    assert!(events.is_empty(), "the stack halts before the next side effect");

    let (outcome, ..) = run(
        json!({"start": {"stack": [
            {"action": {"action": "proxy", "target": "next.md",
                        "with": {"first": "{{ missing }}", "last": "{{ no_such_fn() }}"}}}
        ]}}),
        LifecycleSignal::Start,
        json!({}),
    );
    assert!(outcome.evaluation_error.is_some(), "a failing last member fails the whole overlay");
    assert!(outcome.control.is_none(), "no partial handoff is installed");
}

#[test]
fn an_unavailable_global_fails_with_its_reason_and_never_reads_the_document() {
    // Event-time evaluation alone, without the prepare-time check, still
    // refuses: the document's `err` and `group` are never read instead.
    for (signal, message, root, code) in [
        (LifecycleSignal::Start, "{{ err.msg }}", "err", EVENT_HAS_NO_ERROR),
        (LifecycleSignal::Success, "x {{ err }} y", "err", EVENT_HAS_NO_ERROR),
        (LifecycleSignal::Success, "{{ group.label }}", "group", OUTSIDE_GROUP),
    ] {
        let (outcome, events, ..) = run(
            json!({signal.property_name(): {"stack": [{"action": {"message": message}}]}}),
            signal,
            json!({"err": {"msg": "document"}, "group": {"label": "document"}}),
        );
        assert_eq!(unavailable_read(&outcome), (root.to_string(), code.to_string()), "`{message}`");
        assert!(events.is_empty(), "`{message}` dispatched {events:?}");
    }

    let (outcome, events, ..) = run(
        json!({"start": {"stack": [{"action": {"message": "{{ doc.err.msg }}/{{ doc.group.label }}"}}]}}),
        LifecycleSignal::Start,
        json!({"err": {"msg": "document"}, "group": {"label": "g"}}),
    );
    assert!(outcome.evaluation_error.is_none());
    assert_eq!(events, vec![Emitted::Message("document/g".into())], "`doc.*` reads the document");
}

#[test]
fn malformed_expressions_and_unknown_functions_halt_before_side_effects() {
    // A malformed whole value or guard cannot even be parsed: preparation
    // refuses it, so no event ever runs.
    for item in [
        json!({"action": {"message": "{{ spec ? }}"}}),
        json!({"when": "spec ==", "action": {"message": "guarded"}}),
    ] {
        assert!(
            parse_lifecycle_config(&json!({"success": {"stack": [item]}}), Path::new("t.md")).is_err(),
            "{item}"
        );
    }
    // Anything that parses but fails at event time halts the stack before the
    // action it guards, and before every later one.
    for (when, message) in [
        (None, "x {{ spec ? }}"),
        (None, "{{ no_such_fn(spec) }}"),
        (None, "x {{ no_such_fn(spec) }}"),
        (Some("no_such_fn(spec)"), "guarded"),
    ] {
        let mut item = json!({"action": [{"message": message}, {"message": "after"}]});
        if let Some(when) = when {
            item["when"] = json!(when);
        }
        let (outcome, events, ..) = run(
            json!({"success": {"stack": [item, {"action": {"message": "next item"}}]}}),
            LifecycleSignal::Success,
            json!({"spec": "a.md"}),
        );
        assert!(outcome.evaluation_error.is_some(), "`{when:?}`/`{message}` must fail");
        assert!(events.is_empty(), "`{when:?}`/`{message}` dispatched {events:?}");
    }
}

#[test]
fn successful_output_containing_braces_is_delivered_as_data() {
    let (outcome, events, ..) = run(
        json!({"success": {"stack": [
            {"action": {"message": "{{ body }}"}},
            {"action": {"message": "lit: {{{ raw }}}"}}
        ]}}),
        LifecycleSignal::Success,
        json!({"body": "agent wrote {{ x }} and $(y)"}),
    );
    assert!(outcome.evaluation_error.is_none(), "{:?}", outcome.evaluation_error);
    assert_eq!(
        events,
        vec![
            Emitted::Message("agent wrote {{ x }} and $(y)".into()),
            Emitted::Message("lit: {{ raw }}".into()),
        ]
    );
}

/// A library caller can recover the original Darkmatter failure and the
/// Claudine context it happened in — through the direct outcome, a proxy
/// overlay failure, the lifecycle composition error, and the catch-side
/// wrappers — without the CLI and without parsing rendered text.
#[test]
fn the_typed_darkmatter_cause_survives_every_library_wrapper() {
    fn darkmatter_cause<'a>(error: &'a (dyn std::error::Error + 'static)) -> Option<&'a ExpressionError> {
        let mut current = Some(error);
        while let Some(error) = current {
            if let Some(found) = error.downcast_ref::<ExpressionError>() {
                return Some(found);
            }
            current = error.source();
        }
        None
    }
    let is_unknown_function =
        |error: Option<&ExpressionError>| matches!(error, Some(ExpressionError::UnknownFunction { .. }));

    // Direct: the event outcome.
    let (outcome, ..) = run(
        json!({"success": {"stack": [{"action": {"message": "{{ no_such_fn() }}"}}]}}),
        LifecycleSignal::Success,
        json!({}),
    );
    let info = outcome.evaluation_error.expect("evaluation error");
    assert_eq!(info.property.as_deref(), Some("success.stack[0].action[0]"));
    assert!(is_unknown_function(info.cause.as_ref().and_then(|c| c.expression_error())));

    // Lifecycle: the composition error a caller propagates.
    let error = CompositionError::lifecycle_evaluation("success", Path::new("t.md"), &info);
    assert!(is_unknown_function(error.lifecycle_cause().and_then(|c| c.expression_error())));
    assert!(is_unknown_function(darkmatter_cause(&error)), "reachable through Error::source");
    let CompositionError::LifecycleEvaluationError { event, property, .. } = &error else {
        panic!("{error:?}");
    };
    assert_eq!((event.as_str(), property.as_deref()), ("success", Some("success.stack[0].action[0]")));

    // Catch: the render wrappers and the `err` rebuilt for a routed event.
    let emitted = error.already_emitted();
    assert!(is_unknown_function(emitted.lifecycle_cause().and_then(|c| c.expression_error())));
    let routed = LifecycleErrorInfo::from_composition_error(&emitted);
    assert!(is_unknown_function(routed.cause.as_ref().and_then(|c| c.expression_error())));
    assert!(
        routed.to_value().get("cause").is_none_or(|cause| !cause.to_string().contains("no_such_fn")),
        "the typed cause is not projected into `err.*`"
    );

    // Proxy: an overlay member's failure.
    let (outcome, ..) = run(
        json!({"start": {"stack": [{"action": {"action": "proxy", "target": "next.md",
                                                "with": {"meta": {"bad": "{{ no_such_fn() }}"}}}}]}}),
        LifecycleSignal::Start,
        json!({}),
    );
    let info = outcome.evaluation_error.expect("evaluation error");
    assert_eq!(info.variant, "LifecycleProxyWithEvaluationFailed");
    let cause = info.cause.as_ref().expect("the proxy failure keeps its cause");
    assert!(is_unknown_function(cause.expression_error()));
    assert!(is_unknown_function(darkmatter_cause(cause)), "the cause's source is the Darkmatter error");
}

/// The lifecycle executor renders the shared missing-property and escape table
/// (Darkmatter's `absent_property_contract` inputs) exactly as Darkmatter does,
/// with no extra rejection or evaluation pass of its own.
#[test]
fn event_time_values_follow_the_shared_missing_property_table() {
    for case in crate::composition::interpolation_conformance::missing_property_cases() {
        let (outcome, _, document, _) = run(
            json!({"success": {"stack": [{"action": {"set": {"out": case.input}}}]}}),
            LifecycleSignal::Success,
            Value::Object(case.frontmatter),
        );
        assert!(outcome.evaluation_error.is_none(), "`{}`: {:?}", case.name, outcome.evaluation_error);
        assert_eq!(document.get("out"), Some(&case.expected), "`{}`", case.name);
    }
}
