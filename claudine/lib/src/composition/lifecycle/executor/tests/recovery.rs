//! `try(expr, fallback)` in Claudine lifecycle events: a recoverable read
//! failure selects the fallback in a message, a `set` value, and a `when:`
//! condition, while binding, contract, and shell failures stay what they
//! were.
//!
//! Claudine's executor evaluates through Darkmatter's shared evaluator, so
//! these pin that the recovery reaches every Claudine site unchanged.

use super::*;

use crate::composition::RuntimeState;
use crate::composition::lifecycle::bindings::EVENT_HAS_NO_ERROR;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{BindingError, ExpressionError};

/// Runs `signal`'s event of `lifecycle` for a document in `dir`, with `shell`,
/// returning the outcome, the emissions, and the runtime layer.
fn run_in(
    dir: &Path,
    lifecycle: Value,
    signal: LifecycleSignal,
    shell: &MockShell,
) -> (LifecycleEventOutcome, Vec<Emitted>, RuntimeState) {
    let source_path = dir.join("t.md");
    let config = parse_lifecycle_config(&lifecycle, &source_path).unwrap();
    let base = map(json!({}));
    let live = std::sync::Mutex::new(base.clone().into());
    let runtime = RuntimeState::new();
    let (_engine_dir, engine) = temp_engine();
    let recorder = Recorder::default();
    let harness = Harness::default();
    let context = ctx_with_runtime(signal, &base, &live, &runtime, &engine, shell, &recorder, &harness, &source_path);
    let outcome = context.execute_event(&config);
    (outcome, recorder.events(), runtime)
}

fn run(dir: &Path, lifecycle: Value) -> (LifecycleEventOutcome, Vec<Emitted>, RuntimeState) {
    run_in(dir, lifecycle, LifecycleSignal::Start, &MockShell::new(0))
}

fn messages(texts: &[&str]) -> Vec<Emitted> {
    texts.iter().map(|text| Emitted::Message(text.to_string())).collect()
}

/// The typed Darkmatter error that stopped the event.
fn expression_error(outcome: &LifecycleEventOutcome) -> &ExpressionError {
    let info = outcome.evaluation_error.as_ref().unwrap_or_else(|| panic!("an evaluation error: {outcome:?}"));
    info.cause
        .as_ref()
        .and_then(|cause| cause.expression_error())
        .unwrap_or_else(|| panic!("a typed cause: {info:?}"))
}

const OPTIONAL: &str = "{{ try(load_data('rows.yaml', 'string[]'), ['none']) }}";

#[test]
fn claudine_lifecycle_try_recovers_optional_data() {
    let lifecycle = json!({ "start": [
        { "message": "rows: {{ first(try(load_data('rows.yaml', 'string[]'), ['none'])) }}" },
        { "set": { "rows": OPTIONAL } },
        { "message": "after" },
    ] });

    let missing = tempfile::tempdir().unwrap();
    let (outcome, events, runtime) = run(missing.path(), lifecycle.clone());
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["rows: none", "after"]));
    assert_eq!(runtime.snapshot().mutations.get("rows"), Some(&json!(["none"])));

    // Control: the present file is loaded, not the fallback.
    let present = tempfile::tempdir().unwrap();
    std::fs::write(present.path().join("rows.yaml"), "- a\n- b\n").unwrap();
    let (outcome, events, runtime) = run(present.path(), lifecycle);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["rows: a", "after"]));
    assert_eq!(runtime.snapshot().mutations.get("rows"), Some(&json!(["a", "b"])));

    // Control: unguarded, the same miss stops the event before any message.
    let (outcome, events, _) = run(
        missing.path(),
        json!({ "start": [{ "message": "{{ load_data('rows.yaml', 'string[]') }}" }, { "message": "after" }] }),
    );
    assert!(outcome.evaluation_error.is_some(), "{outcome:?}");
    assert!(events.is_empty(), "{events:?}");
}

/// An unavailable lifecycle global is a binding failure, never a read
/// failure: `try` passes it through and the fallback never renders.
#[test]
fn claudine_lifecycle_try_keeps_binding_failure_fatal() {
    let dir = tempfile::tempdir().unwrap();
    let (outcome, events, _) = run(
        dir.path(),
        json!({ "start": [{ "message": "{{ try(err.msg, 'fallback') }}" }, { "message": "after" }] }),
    );
    match expression_error(&outcome) {
        ExpressionError::Binding(binding) => match binding.as_ref() {
            BindingError::Unavailable(read) => {
                assert_eq!((read.root.as_str(), read.reason.code()), ("err", EVENT_HAS_NO_ERROR));
            }
            other => panic!("an unavailable read: {other:?}"),
        },
        other => panic!("a binding error: {other:?}"),
    }
    assert!(events.is_empty(), "neither the fallback nor a later action ran: {events:?}");

    // Control: an available `err` renders through `try` unchanged.
    let error = LifecycleErrorInfo {
        kind: "ClaudineError",
        variant: "Io".to_string(),
        msg: "it broke".to_string(),
        snapshot: None,
        property: None,
        reason: crate::composition::LifecycleEvaluationReason::Expression,
        cause: None,
    };
    let source_path = dir.path().join("t.md");
    let config =
        parse_lifecycle_config(&json!({ "failure": [{ "message": "{{ try(err.msg, 'fallback') }}" }] }), &source_path)
            .unwrap();
    let fm = map(json!({}));
    let (_engine_dir, engine) = temp_engine();
    let shell = MockShell::new(0);
    let recorder = Recorder::default();
    let harness = Harness::default();
    let outcome = ctx(LifecycleSignal::Failure, &fm, Some(&error), &engine, &shell, &recorder, &harness, &source_path)
        .execute_event(&config);
    assert!(outcome.evaluation_error.is_none(), "{outcome:?}");
    assert_eq!(recorder.events(), messages(&["it broke"]));
}

/// A missing contract import and an unsupported format describe the request,
/// not the optional input, so both stop the event with their own error.
#[test]
fn claudine_lifecycle_try_keeps_contract_failure_fatal() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("rows.yaml"), "- a\n").unwrap();
    std::fs::write(dir.path().join("rows.xyz"), "a\n").unwrap();

    let (outcome, events, _) = run(
        dir.path(),
        json!({ "start": [
            { "message": "{{ try(load_data('rows.yaml', 'Row@./no-such-types.yaml[]'), 'fallback') }}" },
            { "message": "after" },
        ] }),
    );
    assert!(
        matches!(expression_error(&outcome), ExpressionError::Data(error) if matches!(error.as_ref(), DataError::ContractImport(_))),
        "{:?}",
        expression_error(&outcome)
    );
    assert!(events.is_empty(), "{events:?}");

    let (outcome, events, _) = run(
        dir.path(),
        json!({ "start": [
            { "message": "{{ try(load_data('rows.xyz', 'string[]'), 'fallback') }}" },
            { "message": "after" },
        ] }),
    );
    match expression_error(&outcome) {
        ExpressionError::Data(error) => assert_eq!(error.code(), Some("dm.data.unsupported_format")),
        other => panic!("a data error: {other:?}"),
    }
    assert!(events.is_empty(), "{events:?}");

    // Control: the same file under a valid contract loads.
    let (outcome, events, _) =
        run(dir.path(), json!({ "start": [{ "message": "{{ first(try(load_data('rows.yaml', 'string[]'), [])) }}" }] }));
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["a"]));
}

#[test]
fn claudine_when_condition_try_recovers() {
    let lifecycle = json!({ "start": [
        { "when": "length(try(load_data('rows.yaml', 'string[]'), [])) == 0", "then": [{ "message": "no rows" }] },
        { "when": "length(try(load_data('rows.yaml', 'string[]'), [])) > 0", "then": [{ "message": "some rows" }] },
    ] });

    let missing = tempfile::tempdir().unwrap();
    let (outcome, events, _) = run(missing.path(), lifecycle.clone());
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["no rows"]));

    let present = tempfile::tempdir().unwrap();
    std::fs::write(present.path().join("rows.yaml"), "- a\n").unwrap();
    let (outcome, events, _) = run(present.path(), lifecycle);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["some rows"]));

    // Control: the unguarded condition stops the event.
    let (outcome, events, _) = run(
        missing.path(),
        json!({ "start": [{ "when": "length(load_data('rows.yaml', 'string[]')) == 0", "then": [{ "message": "no rows" }] }] }),
    );
    assert!(outcome.evaluation_error.is_some(), "{outcome:?}");
    assert!(events.is_empty(), "{events:?}");
}

/// A failing command is a dispatch failure of its own action. A `try` that
/// recovered a read earlier in the event does not absorb it, and a later
/// action does not run.
#[test]
fn claudine_lifecycle_shell_failure_is_outside_try() {
    let dir = tempfile::tempdir().unwrap();
    let lifecycle = json!({ "start": [
        { "when": "length(try(load_data('rows.yaml', 'string[]'), [])) == 0", "then": [
            { "message": "{{ first(try(load_data('rows.yaml', 'string[]'), ['none'])) }}" },
            { "shell": "false" },
        ] },
        { "message": "after" },
    ] });

    let failing = MockShell::new(1);
    let (outcome, events, _) = run_in(dir.path(), lifecycle.clone(), LifecycleSignal::Start, &failing);
    assert!(outcome.evaluation_error.is_none(), "the shell failure is not an evaluation error: {outcome:?}");
    let info = outcome.action_error.as_ref().unwrap_or_else(|| panic!("the shell failure is reported: {outcome:?}"));
    assert_eq!(info.variant, "shell", "{info:?}");
    assert_eq!(failing.commands(), ["false"]);
    assert_eq!(events, messages(&["none"]), "the recovered message ran; nothing after the failure did");

    // Control: the same stack with a succeeding command runs to the end.
    let passing = MockShell::new(0);
    let (outcome, events, _) = run_in(dir.path(), lifecycle, LifecycleSignal::Start, &passing);
    assert_eq!(outcome, LifecycleEventOutcome::default());
    assert_eq!(events, messages(&["none", "after"]));
}
