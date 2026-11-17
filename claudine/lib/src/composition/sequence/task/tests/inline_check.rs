//! An inline container's `$schema` is a check judged at its turn: after its
//! `when:` admits it, against the enclosing values, setters, and runtime
//! writes so far, and before its run opens. A refusal is an unstarted task
//! failure the enclosing container handles; the check never changes a value.

use super::*;

/// An outer container whose children are `children`, with a `failure` event
/// that announces itself.
fn outer(children: Value) -> Value {
    json!({ "name": "outer", "failure": [{ "notify": "outer-failure" }], "sequence": children })
}

/// An inline container checking `schema`, whose events and one child
/// announce themselves.
fn checked(schema: Value) -> Value {
    json!({
        "name": "inner",
        "$schema": schema,
        "initialize": [{ "notify": "inner-initialize" }],
        "finalize": [{ "notify": "inner-finalize" }],
        "sequence": [{ "name": "work", "shell": "echo inner" }],
    })
}

fn run(step: Value, setters: Option<&Value>) -> (TaskOutcome, Vec<String>, Vec<String>, Arc<RuntimeState>) {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), step);
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new_sequence());
    let wiring = Wiring { runtime: Some(&runtime), user_setters: setters, ..Wiring::new(&recorder, &shell) };
    let outcome = fixture.execute(&wiring);
    (outcome, recorder.events(), shell.commands(), runtime)
}

#[test]
fn an_earlier_set_satisfies_the_check() {
    let (outcome, events, commands, _) = run(
        outer(json!([
            { "name": "write", "side_effect": { "set": { "reviewer": "ada" } } },
            checked(json!({ "reviewer": "string(required)" })),
        ])),
        None,
    );
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands, vec!["echo inner".to_string()]);
    assert!(events.contains(&"notify:inner-initialize".to_string()), "{events:?}");
}

#[test]
fn a_caller_setter_satisfies_the_check() {
    let setters = json!({ "reviewer": "ada" });
    let (outcome, _, commands, _) =
        run(outer(json!([checked(json!({ "reviewer": "string(required)" }))])), Some(&setters));
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands.len(), 1);
}

#[test]
fn a_missing_value_refuses_the_container_before_it_opens_and_the_ancestor_handles_it() {
    let (outcome, events, commands, runtime) = run(
        outer(json!([
            checked(json!({ "reviewer": "string(required)" })),
            { "name": "after", "shell": "echo after" },
        ])),
        None,
    );
    assert!(!outcome.succeeded());
    let message = failure_message(&outcome);
    assert!(message.contains("sequence container `tasks[0].sequence[0]`"), "{message}");
    assert!(message.contains("failed its `$schema` check at `reviewer`"), "{message}");
    // Nothing of the refused container ran, and the outer container stopped.
    assert!(commands.is_empty(), "{commands:?}");
    assert!(!events.iter().any(|event| event.starts_with("notify:inner-")), "{events:?}");
    assert!(events.contains(&"notify:outer-failure".to_string()), "{events:?}");
    assert!(runtime.mutations().is_empty());
}

#[test]
fn a_wrong_typed_value_is_refused_with_its_property() {
    let (outcome, _, commands, _) = run(
        outer(json!([
            { "name": "write", "side_effect": { "set": { "rank": "high" } } },
            checked(json!({ "rank": "number(required)" })),
        ])),
        None,
    );
    let message = failure_message(&outcome);
    assert!(message.contains("failed its `$schema` check at `/rank`") || message.contains("at `rank`"), "{message}");
    assert!(commands.is_empty());
}

#[test]
fn a_false_guard_skips_the_check() {
    let mut container = checked(json!({ "reviewer": "string(required)" }));
    container["when"] = json!("false");
    let (outcome, events, commands, _) = run(outer(json!([container])), None);
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert!(commands.is_empty());
    assert!(!events.iter().any(|event| event.starts_with("notify:inner-")), "{events:?}");
}

#[test]
fn a_defaulted_or_converted_validation_copy_never_changes_the_enclosing_values() {
    let (outcome, _, commands, runtime) = run(
        outer(json!([
            { "name": "write", "side_effect": { "set": { "rank": "3" } } },
            checked(json!({ "rank": "number", "level": "string(default(high))" })),
        ])),
        None,
    );
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands.len(), 1);
    let mutations = runtime.mutations();
    assert_eq!(mutations.get("rank"), Some(&json!("3")), "the written text stays text");
    assert!(!mutations.contains_key("level"), "a default is never committed: {mutations:?}");
}

/// A preview runner: containers compose without running their events.
struct DryRun;

impl PromptTaskRunner for DryRun {
    fn is_dry_run(&self) -> bool {
        true
    }
    fn run(&self, _request: &PromptTaskRequest) -> Result<PromptRunOutcome, CompositionError> {
        unreachable!("the fixture launches no prompt")
    }
}

#[test]
fn a_dry_run_names_the_check_without_judging_it() {
    let dir = TempDir::new().unwrap();
    let source = one_step_source(dir.path(), outer(json!([checked(json!({ "reviewer": "string(required)" }))])));
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new_sequence());
    let outcome = fixture.execute(&Wiring { runtime: Some(&runtime), prompt: &DryRun, ..Wiring::new(&recorder, &shell) });
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    let notes: Vec<_> = recorder.events().into_iter().filter(|event| event.contains("checks its `$schema`")).collect();
    assert_eq!(notes, vec!["info:`tasks[0].sequence[0]` checks its `$schema` at its turn (not evaluated in a dry run)".to_string()]);
}

/// Each loop pass enters through the existing `start` preparation, which
/// judges the check again against that pass's values; no check runs between
/// the children of one pass.
#[test]
fn each_loop_pass_judges_the_check_against_its_own_values() {
    let container = json!({
        "name": "inner",
        "$schema": { "n": "number(max(1))" },
        "loop": { "max": 3, "until": "loop.count >= 3", "action": "increment(n)" },
        "sequence": [{ "name": "work", "shell": "echo pass" }, { "name": "again", "shell": "echo again" }],
    });
    let dir = TempDir::new().unwrap();
    let source = write_source(dir.path(), "seq.md", &[("n", json!(0)), ("sequence", json!([outer(json!([container]))]))], "Body.\n");
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let shell = FakeTaskShell::default();
    let runtime = Arc::new(RuntimeState::new_sequence());
    let outcome = fixture.execute(&Wiring { runtime: Some(&runtime), ..Wiring::new(&recorder, &shell) });
    let message = failure_message(&outcome);
    assert!(message.contains("failed its `$schema` check"), "{message}");
    // Passes 1 and 2 see n = 0 and 1 and run both children; pass 3 sees 2.
    assert_eq!(shell.commands().len(), 4, "{:?}", shell.commands());
}

/// An explicit `null` is judged by the declared type, as a missing value
/// is, while an empty string the type admits passes.
#[test]
fn an_explicit_null_is_refused_and_an_admitted_empty_value_passes() {
    for (value, admitted) in [(json!(null), false), (json!(""), true)] {
        let (outcome, _, commands, _) = run(
            outer(json!([
                { "name": "write", "side_effect": { "set": { "reviewer": value.clone() } } },
                checked(json!({ "reviewer": "string(required)" })),
            ])),
            None,
        );
        assert_eq!(outcome.succeeded(), admitted, "{value}: {}", failure_message(&outcome));
        assert_eq!(commands.len(), usize::from(admitted), "{value}");
        if !admitted {
            assert!(failure_message(&outcome).contains("failed its `$schema` check"), "{}", failure_message(&outcome));
        }
    }
}
