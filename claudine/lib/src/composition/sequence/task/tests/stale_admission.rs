//! A container is admitted against its inline `$schema` as the declaring
//! file reads at its turn, not as the approved graph froze it: a declaration
//! an earlier task tightened refuses the container before its `initialize`
//! runs, wherever the container sits (a nested child, a group leg, a
//! `kind: task` file, or a referenced sequence document).

use super::*;

/// The check every container here declares, and the edit that tightens it so
/// the written `rank: 1` fails.
const LOOSE: &str = "number(min(0))";
const TIGHT: &str = "number(max(0))";

/// Runs every child command successfully; when it runs `edit`, replaces the
/// first `from` in `path` with `to`.
struct EditingShell {
    path: PathBuf,
    from: String,
    to: String,
}

impl TaskShellRunner for EditingShell {
    fn run(&self, command: &str, _timeout: Duration, _interrupt: Option<&AtomicBool>,
        _live: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError>
    {
        if command == "edit" {
            let text = fs::read_to_string(&self.path).unwrap();
            assert!(text.contains(&self.from), "{} holds no `{}`:\n{text}", self.path.display(), self.from);
            fs::write(&self.path, text.replacen(&self.from, &self.to, 1)).unwrap();
        }
        Ok(ShellCommandOutput { stdout: command.into(), exit_code: 0, timed_out: false, interrupted: false, runaway: None })
    }
}

/// An outer container whose failure handler announces itself.
fn outer(children: Value) -> Value {
    json!({ "name": "outer", "failure": [{ "notify": "outer-failure" }], "sequence": children })
}

fn write_rank() -> Value {
    json!({ "name": "write", "side_effect": { "set": { "rank": 1 } } })
}

fn edit() -> Value {
    json!({ "name": "edit", "shell": "edit" })
}

/// An inline container checking `rank` against `contract`, whose
/// `initialize` announces itself and writes `initialized`.
fn checked(contract: &str) -> Value {
    json!({
        "name": "inner",
        "$schema": { "rank": contract },
        "initialize": [{ "notify": "inner-initialize" }, { "set": { "initialized": true } }],
        "finalize": [{ "notify": "inner-finalize" }],
        "sequence": [{ "name": "work", "shell": "work" }],
    })
}

struct Run {
    outcome: TaskOutcome,
    events: Vec<String>,
    runtime: Arc<RuntimeState>,
}

/// Run the one-step sequence `seq.md` holding `step`, editing `edited` (a
/// file in the fixture directory) when the `edit` child runs. `files` are
/// written beside it first.
fn run(files: &[(&str, Value)], step: Value, edited: &str, from: &str, to: &str) -> Run {
    let dir = TempDir::new().unwrap();
    for (name, value) in files {
        write_yaml(dir.path(), name, value);
    }
    let source = one_step_source(dir.path(), step);
    let path = dir.path().join(edited);
    let fixture = Fixture::build(dir, &source).unwrap();
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let shell = EditingShell { path, from: from.into(), to: to.into() };
    let runtime = Arc::new(RuntimeState::new_sequence());
    let mut wiring = Wiring { runtime: Some(&runtime), ..Wiring::new(&recorder, &fake) };
    wiring.shell = &shell;
    let outcome = fixture.execute(&wiring);
    Run { outcome, events: recorder.events(), runtime }
}

/// The refused container ran no event, and its `initialize` wrote nothing.
fn assert_refused_before_initialize(run: &Run) {
    assert!(!run.outcome.succeeded());
    let message = failure_message(&run.outcome);
    assert!(message.contains("failed its `$schema` check"), "{message}");
    assert!(!run.events.iter().any(|event| event.starts_with("notify:inner-")), "{:?}", run.events);
    assert!(run.events.contains(&"notify:outer-failure".to_string()), "{:?}", run.events);
    assert!(!run.runtime.mutations().contains_key("initialized"), "{:?}", run.runtime.mutations());
}

#[test]
fn a_check_tightened_by_an_earlier_sibling_refuses_the_container_before_initialize() {
    let run = run(&[], outer(json!([write_rank(), edit(), checked(LOOSE)])), "seq.md", LOOSE, TIGHT);
    assert_refused_before_initialize(&run);
}

#[test]
fn a_tightened_check_on_a_group_leg_refuses_the_leg_before_initialize() {
    let group = json!({ "name": "legs", "group": { "name": "legs", "tasks": [checked(LOOSE)] } });
    let run = run(&[], outer(json!([write_rank(), edit(), group])), "seq.md", LOOSE, TIGHT);
    // A failed leg withholds its mutations anyway; its events are the proof.
    assert_refused_before_initialize(&run);
}

#[test]
fn a_tightened_check_in_a_task_file_refuses_the_container_before_initialize() {
    let mut file = checked(LOOSE);
    file["kind"] = json!("task");
    let step = json!({ "name": "inner", "task": "task.yaml" });
    let run = run(&[("task.yaml", file)], outer(json!([write_rank(), edit(), step])), "task.yaml", LOOSE, TIGHT);
    assert_refused_before_initialize(&run);
}

#[test]
fn a_tightened_check_in_a_referenced_document_refuses_its_child_before_initialize() {
    let child = json!({ "kind": "sequence", "sequence": [checked(LOOSE)] });
    let step = json!({ "name": "nested", "sequence": "child.yaml" });
    let run = run(&[("child.yaml", child)], outer(json!([write_rank(), edit(), step])), "child.yaml", LOOSE, TIGHT);
    assert_refused_before_initialize(&run);
}

#[test]
fn a_check_loosened_by_an_earlier_sibling_admits_the_container() {
    let run = run(&[], outer(json!([write_rank(), edit(), checked(TIGHT)])), "seq.md", TIGHT, LOOSE);
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert!(run.events.contains(&"notify:inner-initialize".to_string()), "{:?}", run.events);
    assert_eq!(run.runtime.mutations().get("initialized"), Some(&json!(true)));
}

#[test]
fn an_unchanged_rejecting_check_refuses_the_container_before_initialize() {
    let run = run(&[], outer(json!([write_rank(), edit(), checked(TIGHT)])), "seq.md", "work", "work");
    assert_refused_before_initialize(&run);
}

#[test]
fn a_false_guard_neither_rereads_nor_judges_the_check() {
    let mut container = checked(LOOSE);
    container["when"] = json!("false");
    let run = run(&[], outer(json!([write_rank(), edit(), container])), "seq.md", LOOSE, TIGHT);
    assert!(run.outcome.succeeded(), "{}", failure_message(&run.outcome));
    assert!(!run.events.iter().any(|event| event.starts_with("notify:inner-")), "{:?}", run.events);
}

/// A declaration edited into one that no longer reads is a typed declaration
/// error at admission, not a value refusal, and still starts nothing.
#[test]
fn an_edited_malformed_declaration_is_a_declaration_error_at_admission() {
    let run = run(&[], outer(json!([write_rank(), edit(), checked(LOOSE)])), "seq.md",
        &format!(r#"{{"rank":"{LOOSE}"}}"#), "[]");
    assert!(!run.outcome.succeeded());
    let message = failure_message(&run.outcome);
    assert!(message.contains("invalid sequence declaration `tasks[0].sequence[2].$schema`"), "{message}");
    assert!(!message.contains("failed its `$schema` check"), "{message}");
    assert!(!run.events.iter().any(|event| event.starts_with("notify:inner-")), "{:?}", run.events);
    assert!(run.events.contains(&"notify:outer-failure".to_string()), "{:?}", run.events);
}

/// Admission reuses the invocation's prepared declaration while the file and
/// its declaration are unchanged, and prepares the edited one once.
#[cfg(feature = "work-counters")]
#[test]
fn admission_prepares_only_an_edited_declaration() {
    use crate::work_counters::{Work, snapshot};
    for (from, to, preparations) in [("work", "work", 0), (LOOSE, TIGHT, 1)] {
        let before = snapshot();
        let run = run(&[], outer(json!([write_rank(), edit(), checked(LOOSE)])), "seq.md", from, to);
        let work = snapshot().since(&before);
        // `Fixture::build` prepared the frozen check once.
        assert_eq!(work.get(Work::SequenceSchemaPrepare), 1 + preparations, "{from} -> {to}: {}", failure_message(&run.outcome));
    }
}

/// Admission judges the values before `initialize`; the first `start` still
/// judges them after it, so a value `initialize` writes is checked before any
/// child runs.
#[test]
fn the_first_start_still_judges_what_initialize_wrote() {
    let mut container = checked(LOOSE);
    container["initialize"] = json!([{ "notify": "inner-initialize" }, { "set": { "rank": -1 } }]);
    let run = run(&[], outer(json!([write_rank(), edit(), container])), "seq.md", "work", "work");
    assert!(!run.outcome.succeeded());
    let message = failure_message(&run.outcome);
    assert!(message.contains("failed its `$schema` check at `/rank`") || message.contains("at `rank`"), "{message}");
    // Admitted: its `initialize` ran, and the refusal blocked the started
    // container before its child.
    assert!(run.events.contains(&"notify:inner-initialize".to_string()), "{:?}", run.events);
    assert!(run.events.contains(&"notify:inner-finalize".to_string()), "{:?}", run.events);
}
