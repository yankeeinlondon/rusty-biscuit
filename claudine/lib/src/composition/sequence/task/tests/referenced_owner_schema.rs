//! A whole referenced sequence document is prepared where it runs: its events
//! and children read its prepared values (its own properties, coerced and
//! defaulted by its `$schema`, under the setters and the runtime writes so
//! far), and its schema judges them each time the container starts, after
//! its own `initialize`. A refusal fails the task before any child runs.

use super::*;

/// An outer container whose children are `children`.
fn outer(children: Value) -> Value {
    json!({ "name": "outer", "sequence": children })
}

fn write_rank(value: Value) -> Value {
    json!({ "name": "write", "side_effect": { "set": { "rank": value } } })
}

/// The referenced document: a required numeric `rank`, authored valid, and
/// one shell child; `extra` adds root keys.
fn ranked(extra: Value) -> Value {
    let mut document = json!({
        "kind": "sequence",
        "$schema": { "rank": "number(required)" },
        "rank": 1,
        "sequence": [{ "name": "observe", "shell": "echo observed" }],
    });
    document.as_object_mut().unwrap().extend(extra.as_object().cloned().unwrap_or_default());
    document
}

fn run(child: &Value, step: Value, setters: Option<&Value>) -> (TaskOutcome, Vec<String>, Vec<String>) {
    let dir = TempDir::new().unwrap();
    write_yaml(dir.path(), "child.yaml", child);
    let source = one_step_source(dir.path(), step);
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new_sequence());
    let wiring = Wiring { runtime: Some(&runtime), user_setters: setters, ..Wiring::new(&recorder, &shell) };
    let outcome = fixture.execute(&wiring);
    (outcome, recorder.events(), shell.commands())
}

#[test]
fn an_earlier_invalid_write_fails_the_referenced_document_before_its_child_runs() {
    let child = ranked(json!({ "finalize": [{ "notify": "child-finalize" }] }));
    let (outcome, events, commands) = run(
        &child, outer(json!([write_rank(json!("bad")), { "name": "nested", "sequence": "child.yaml" }])), None,
    );
    assert_eq!(outcome.status, TaskStatus::Failed);
    let message = failure_message(&outcome);
    assert!(message.contains("/rank") && message.contains("child.yaml"), "{message}");
    assert!(commands.is_empty(), "{commands:?}");
    // The container had started: its cleanup still runs.
    assert!(events.contains(&"notify:child-finalize".to_string()), "{events:?}");
}

#[test]
fn a_valid_write_runs_the_referenced_document() {
    let (outcome, _, commands) = run(
        &ranked(json!({})), outer(json!([write_rank(json!(2)), { "name": "nested", "sequence": "child.yaml" }])), None,
    );
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands, vec!["echo observed".to_string()]);
}

#[test]
fn an_invalid_setter_fails_the_referenced_document() {
    let setters = json!({ "rank": "bad" });
    let (outcome, _, commands) = run(&ranked(json!({})), outer(json!([{ "name": "nested", "sequence": "child.yaml" }])), Some(&setters));
    assert_eq!(outcome.status, TaskStatus::Failed);
    assert!(failure_message(&outcome).contains("/rank"), "{}", failure_message(&outcome));
    assert!(commands.is_empty());
}

#[test]
fn the_documents_initialize_runs_before_the_verdict_and_may_correct_a_value() {
    let child = ranked(json!({ "initialize": [{ "set": { "rank": 2 } }] }));
    let (outcome, _, commands) = run(
        &child, outer(json!([write_rank(json!("bad")), { "name": "nested", "sequence": "child.yaml" }])), None,
    );
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands.len(), 1);
}

#[test]
fn events_and_children_read_the_documents_prepared_values() {
    let child = json!({
        "kind": "sequence",
        "$schema": { "who": "string(default(ada))", "rank": "number" },
        "rank": "3",
        "initialize": [{ "notify": "initialize {{ who }} {{ rank + 1 }}" }],
        "start": [{ "notify": "start {{ who }} {{ rank + 1 }}" }],
        "sequence": [{ "name": "inner", "sequence": [], "start": [{ "notify": "child {{ who }} {{ rank + 1 }}" }] }],
    });
    let (outcome, events, _) = run(&child, outer(json!([{ "name": "nested", "sequence": "child.yaml" }])), None);
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(events, vec!["notify:initialize ada 4", "notify:start ada 4", "notify:child ada 4"]);
}

/// A container proxy's target is a whole document: at its `start` its root
/// `$schema` is its document schema (defaults fill, values convert, and the
/// verdict judges its own authored `rank`), never an inline container's
/// check against the proxying caller's values.
fn proxied_target(rank: Value) -> (TaskOutcome, Vec<String>, Vec<String>) {
    let dir = TempDir::new().unwrap();
    write_yaml(dir.path(), "target.yaml", &json!({
        "kind": "sequence",
        "$schema": { "rank": "number(required)", "who": "string(default(ada))" },
        "rank": rank,
        "start": [{ "info": "target {{ who }} {{ rank + 1 }}" }],
        "sequence": [{ "name": "target", "shell": "target" }],
    }));
    let source = write_source(dir.path(), "root.md", &[("sequence", json!([
        { "name": "source", "sequence": [{ "name": "bad", "shell": "bad" }], "failure": [{ "proxy": "target.yaml" }] },
        { "name": "preapproved-target", "sequence": "target.yaml" },
    ]))], "Body.");
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default().stdout_for("bad", "partial").failing("bad");
    let runtime = Arc::new(RuntimeState::new_sequence());
    let outcome = fixture.execute(&Wiring { runtime: Some(&runtime), ..Wiring::new(&recorder, &shell) });
    (outcome, recorder.events(), shell.commands())
}

#[test]
fn a_proxy_target_starts_as_a_whole_document_with_its_prepared_values() {
    let (outcome, events, commands) = proxied_target(json!("3"));
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(events, vec!["info:target ada 4"]);
    assert_eq!(commands, vec!["bad".to_string(), "target".to_string()]);
}

#[test]
fn a_proxy_targets_document_schema_judges_its_own_values_at_start() {
    let (outcome, _, commands) = proxied_target(json!("bad"));
    assert_eq!(outcome.status, TaskStatus::Failed);
    let message = failure_message(&outcome);
    assert!(message.contains("/rank") && message.contains("target.yaml"), "{message}");
    assert_eq!(commands, vec!["bad".to_string()]);
}
