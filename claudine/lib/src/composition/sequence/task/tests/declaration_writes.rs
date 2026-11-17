//! No value route writes a sequence declaration at execution time: a prompt
//! task's `with:` naming `$state` or `template` fails before the provider
//! runs, and a runtime `set` batch naming one is refused whole, so none of
//! its other writes publish.

use super::*;

#[test]
fn a_with_key_naming_a_declaration_fails_before_the_provider_runs() {
    for key in ["$state", "template", "template.desc"] {
        let dir = TempDir::new().unwrap();
        write_source(dir.path(), "review.md", &[("title", json!("Review"))], "Review.\n");
        let source = one_step_source(dir.path(), json!({ "name": "alpha", "prompt": "review.md", "with": { key: {} } }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let prompt = FakePrompt::succeeding("never");
        let mut wiring = Wiring::new(&recorder, &shell);
        wiring.prompt = &prompt;

        let outcome = fixture.execute(&wiring);

        assert_eq!(outcome.status, TaskStatus::Failed, "{key}");
        let message = failure_message(&outcome);
        assert!(message.contains(&format!("`with.{key}` in")), "{key}: {message}");
        assert!(prompt.requests.lock().unwrap().is_empty(), "{key}: the provider never ran");
    }
}

#[test]
fn a_set_batch_naming_a_declaration_publishes_none_of_its_writes() {
    let run = |set: Value| {
        let dir = TempDir::new().unwrap();
        let source = one_step_source(
            dir.path(),
            json!({ "name": "outer", "sequence": [
                { "name": "write", "side_effect": { "set": set } },
                { "name": "after", "shell": "echo after" },
            ] }),
        );
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();
        let runtime = Arc::new(RuntimeState::new_sequence());
        let outcome = fixture.execute(&Wiring { runtime: Some(&runtime), ..Wiring::new(&recorder, &shell) });
        (outcome, shell.commands(), runtime.mutations())
    };
    // Control: the same batch without the declaration publishes both writes.
    let (outcome, commands, mutations) = run(json!({ "reviewer": "ada", "rank": 2 }));
    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(commands, ["echo after"]);
    assert_eq!((mutations.get("reviewer"), mutations.get("rank")), (Some(&json!("ada")), Some(&json!(2))));

    for declaration in ["template", "$state"] {
        let (outcome, commands, mutations) = run(json!({ "reviewer": "ada", declaration: {}, "rank": 2 }));
        assert!(!outcome.succeeded(), "{declaration}");
        assert!(failure_message(&outcome).contains(declaration), "{declaration}: {}", failure_message(&outcome));
        assert!(mutations.is_empty(), "{declaration}: a refused batch publishes nothing: {mutations:?}");
        assert!(commands.is_empty(), "{declaration}: the container stopped");
    }
}
