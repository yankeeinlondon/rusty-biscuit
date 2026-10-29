//! A group start decides whether to re-prepare its group by rereading only
//! the files that group's preparation reads ([`PreflightGroup::sources`]):
//! its authoring document and the files its own subtree references. A change
//! to the group's own file is still picked up; a change to another group's
//! file neither re-prepares this group nor alters its result.

use super::*;
#[cfg(feature = "work-counters")]
use crate::work_counters::{Work, snapshot};

/// Group file `g{index}.yaml`: announces `g{index} {version}` at `start` and
/// has one leg running `run-{index}`.
fn group_file(index: usize, version: &str) -> Value {
    json!({
        "kind": "group",
        "name": format!("g{index}"),
        "start": [{ "info": format!("g{index} {version}") }],
        "tasks": [{ "name": "leg", "shell": format!("run-{index}") }],
    })
}

/// A one-step sequence whose step holds `groups` nested group steps, step
/// `s{k}` naming its own file `g{k}.yaml`.
fn source_with(dir: &Path, groups: usize) -> String {
    for index in 0..groups {
        write_yaml(dir, &format!("g{index}.yaml"), &group_file(index, "v1"));
    }
    let children: Vec<Value> = (0..groups)
        .map(|index| json!({ "name": format!("s{index}"), "group": format!("g{index}.yaml") }))
        .collect();
    one_step_source(dir, json!({ "name": "outer", "sequence": children }))
}

/// The groups of the fixture's nested sequence, in step order.
fn groups(fixture: &Fixture) -> Vec<&PreflightGroup> {
    let PreflightAction::Sequence(sequence) = &fixture.task().action else { panic!("the step is a sequence") };
    sequence.steps.iter().map(|step| match &step.task.as_ref().expect("a group step").action {
        PreflightAction::Group(group) => group,
        other => panic!("expected a group, got {other:?}"),
    }).collect()
}

/// Runs every command successfully; when it runs `trigger`, writes
/// `replacement` over `path`.
struct RewritingShell {
    trigger: &'static str,
    path: PathBuf,
    replacement: String,
}

impl TaskShellRunner for RewritingShell {
    fn run(&self, command: &str, _timeout: Duration, _interrupt: Option<&AtomicBool>,
        _live: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError>
    {
        if command == self.trigger {
            fs::write(&self.path, &self.replacement).unwrap();
        }
        Ok(ShellCommandOutput { stdout: command.into(), exit_code: 0, timed_out: false, interrupted: false, runaway: None })
    }
}

/// Run the fixture's step with `run-0` writing `replacement` over `file`.
fn run_rewriting(fixture: &Fixture, file: &str, replacement: Value) -> (TaskOutcome, Vec<String>) {
    let shell = RewritingShell {
        trigger: "run-0",
        path: fixture._dir.path().join(file),
        replacement: serde_json::to_string_pretty(&replacement).unwrap(),
    };
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell;
    let outcome = fixture.execute(&wiring);
    (outcome, recorder.events())
}

#[test]
fn each_group_records_its_authoring_document_and_its_own_file() {
    let dir = TempDir::new().unwrap();
    let source = source_with(dir.path(), 3);
    let fixture = Fixture::build(dir, &source).unwrap();

    for (index, group) in groups(&fixture).into_iter().enumerate() {
        let names: Vec<String> = group.sources.iter()
            .map(|path| path.file_name().unwrap().to_string_lossy().into_owned()).collect();
        assert_eq!(names, vec!["seq.md".to_string(), format!("g{index}.yaml")], "group {index}");
        assert!(fixture.graph.sources_unchanged(&group.sources), "group {index}'s files are as audited");
    }
}

#[test]
fn a_rewrite_of_a_groups_own_file_is_seen_at_its_start() {
    let dir = TempDir::new().unwrap();
    let source = source_with(dir.path(), 3);
    let fixture = Fixture::build(dir, &source).unwrap();

    let (outcome, events) = run_rewriting(&fixture, "g1.yaml", group_file(1, "v2"));

    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(events, vec!["info:g0 v1", "info:g1 v2", "info:g2 v1"]);
}

#[test]
fn a_rewrite_of_another_groups_file_leaves_this_group_as_audited() {
    let dir = TempDir::new().unwrap();
    let source = source_with(dir.path(), 3);
    let fixture = Fixture::build(dir, &source).unwrap();

    let (outcome, events) = run_rewriting(&fixture, "g2.yaml", group_file(2, "v2"));

    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(events, vec!["info:g0 v1", "info:g1 v1", "info:g2 v2"]);
    // The decision each start made: only `g2` re-prepares. Before the fix a
    // rewrite of any captured file re-prepared every later group.
    let decisions: Vec<bool> = groups(&fixture).into_iter()
        .map(|group| fixture.graph.sources_unchanged(&group.sources)).collect();
    assert_eq!(decisions, vec![true, true, false]);
}

/// Each group start rereads its own two files, however many other groups
/// the sequence holds (before the fix, every captured file: `groups + 1`).
#[cfg(feature = "work-counters")]
#[test]
fn a_group_start_rereads_only_its_own_files_at_10_and_100() {
    for count in [10, 100] {
        let dir = TempDir::new().unwrap();
        let source = source_with(dir.path(), count);
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let before = snapshot();
        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let expected: Vec<String> = (0..count).map(|index| format!("info:g{index} v1")).collect();
        assert_eq!(recorder.events(), expected, "every group started as audited");
        assert_eq!(work.get(Work::FreshnessSourceReads), 2 * count as u64, "{count} group starts read 2 files each");
    }
}
