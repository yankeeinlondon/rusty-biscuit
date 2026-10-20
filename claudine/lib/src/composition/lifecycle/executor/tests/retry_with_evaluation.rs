//! `retry.with` event-time evaluation into the overlay a retry carries.
//!
//! `retry.with` shares `proxy.with`'s evaluation path, so these tests pin what
//! differs (the overlay rides on [`StackControl::Retry`], the diagnostic names
//! `retry` and the current document) and re-prove the shared rules on the
//! retry surface: native values, literal data, exact nested paths, and that
//! `no_error` cannot hide a failure.

use super::*;

/// Run the `failure` stack of `config` against `state`, as the document
/// `router.md`.
fn run_failure(config: Value, state: Value, err: Option<&LifecycleErrorInfo>) -> LifecycleEventOutcome {
    let config = parse_lifecycle_config(&config, Path::new("router.md")).expect("config parses");
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

/// The overlay of a stack run that is expected to retry.
fn retry_overlay(outcome: &LifecycleEventOutcome) -> &IndexMap<String, Value> {
    match outcome.control.as_ref() {
        Some(StackControl::Retry { overlay, .. }) => overlay,
        other => panic!("expected a retry, got: {other:?}\n{outcome:?}"),
    }
}

/// A `failure` stack with one retry carrying `with`.
fn retry_stack(with: Value) -> Value {
    json!({ "failure": [{ "retry": { "max_attempts": 2, "with": with } }] })
}

#[test]
fn a_retry_overlay_keeps_native_values_static_keys_and_literal_data() {
    let outcome = run_failure(
        retry_stack(json!({
            "flag": "{{ src_bool }}",
            "count": "{{ src_number }}",
            "list": "{{ src_array }}",
            "mixed": "attempt {{ src_number }}",
            "static": "plain text",
            "nested": { "inner": "{{ src_bool }}", "n": 3 },
            "carried": "{{ payload }}",
            "nothing": null,
        })),
        json!({
            "src_bool": false,
            "src_number": 7,
            "src_array": [1, "two"],
            "payload": "{{ target_side }}",
        }),
        None,
    );

    let overlay = retry_overlay(&outcome);
    assert_eq!(overlay.get("flag"), Some(&json!(false)));
    assert_eq!(overlay.get("count"), Some(&json!(7)));
    assert_eq!(overlay.get("list"), Some(&json!([1, "two"])));
    assert_eq!(overlay.get("mixed"), Some(&json!("attempt 7")));
    assert_eq!(overlay.get("static"), Some(&json!("plain text")));
    assert_eq!(overlay.get("nested"), Some(&json!({ "inner": false, "n": 3 })));
    assert_eq!(
        overlay.get("carried"),
        Some(&json!("{{ target_side }}")),
        "template text held in frontmatter is data, not a second template"
    );
    assert_eq!(overlay.get("nothing"), Some(&Value::Null));
    assert_eq!(overlay.len(), 8, "every static top-level key is kept");
}

#[test]
fn an_omitted_or_empty_retry_overlay_is_empty_and_the_other_parameters_are_unchanged() {
    for config in [
        json!({ "failure": [{ "retry": { "max_attempts": 2, "delay": "1s", "backoff": "exponential" } }] }),
        json!({ "failure": [{ "retry": { "max_attempts": 2, "delay": "1s", "backoff": "exponential", "with": {} } }] }),
    ] {
        let outcome = run_failure(config.clone(), json!({}), None);
        assert_eq!(
            outcome.control,
            Some(StackControl::Retry {
                max_attempts: 2,
                backoff: RetryBackoff::Exponential,
                delay: "1s".to_string(),
                overlay: IndexMap::new(),
            }),
            "{config}"
        );
    }
}

#[test]
fn the_err_global_feeds_a_failure_retry_overlay() {
    let err = LifecycleErrorInfo::from_action_failure("shell", "boom");
    let outcome = run_failure(retry_stack(json!({ "reason": "{{ err.msg }}" })), json!({}), Some(&err));
    assert_eq!(retry_overlay(&outcome).get("reason"), Some(&json!("boom")));
}

#[test]
fn a_retry_overlay_failure_names_retry_the_current_document_and_the_nested_path() {
    let outcome = run_failure(
        json!({
            "failure": [{
                "when": "true",
                "then": [{ "retry": { "with": {
                    "secret": "s3cret-value",
                    "metadata": { "area": ["ok", "{{ missing_root() }}"] },
                } } }],
            }],
        }),
        json!({}),
        None,
    );

    assert!(outcome.control.is_none(), "a failed overlay leaves no retry to dispatch");
    let info = outcome.evaluation_error.expect("an unknown function raises");
    assert_eq!(info.variant, "LifecycleWithEvaluationFailed");
    assert!(
        info.msg.contains("failure[0].then[0].retry.with.metadata.area[1]"),
        "roots at the exact authored path: {}",
        info.msg
    );
    assert!(info.msg.contains("`retry` to `router.md`"), "names the verb and the current document: {}", info.msg);
    assert!(!info.msg.contains("s3cret-value"), "no unrelated value is echoed: {}", info.msg);
}

#[test]
fn a_failing_retry_overlay_is_atomic() {
    let outcome = run_failure(
        retry_stack(json!({ "good": "{{ present }}", "bad": "{{ absent_root() }}" })),
        json!({ "present": "value" }),
        None,
    );
    assert!(outcome.control.is_none(), "one failing value aborts the whole retry");
    assert!(outcome.evaluation_error.is_some());
}

#[test]
fn no_error_does_not_suppress_a_retry_overlay_failure() {
    let outcome = run_failure(
        json!({ "failure": [{ "retry": { "no_error": true, "with": { "x": "{{ absent_root() }}" } } }] }),
        json!({}),
        None,
    );
    assert!(outcome.control.is_none(), "`no_error` must not smuggle a retry through");
    assert!(outcome.action_error.is_none(), "an overlay raise is not a dispatch error");
    assert_eq!(
        outcome.evaluation_error.expect("still raises under no_error").variant,
        "LifecycleWithEvaluationFailed"
    );
}

#[test]
fn run_time_template_text_cannot_become_lifecycle_configuration_through_a_retry() {
    let outcome = run_failure(
        retry_stack(json!({ "success": "{{ payload }}" })),
        json!({ "payload": { "info": "{{ secret }}" } }),
        None,
    );
    assert!(outcome.control.is_none());
    let info = outcome.evaluation_error.expect("template text in a lifecycle key fails closed");
    assert_eq!(info.variant, "LifecycleWithEvaluationFailed");
    assert!(info.msg.contains("{{ secret }}"), "{}", info.msg);
}
