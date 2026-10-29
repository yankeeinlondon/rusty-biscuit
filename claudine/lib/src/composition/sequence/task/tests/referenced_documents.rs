//! A referenced data document (a group catalog, a referenced sequence file)
//! is parsed once per distinct text in an invocation and shared by every
//! reference to it: a step that names one catalog entry neither copies the
//! catalog nor parses it again, and a file a task rewrote is parsed again and
//! seen by the next preparation.

use super::*;
#[cfg(feature = "work-counters")]
use crate::work_counters::{Work, nodes, snapshot};

/// A catalog of `entries` groups. Group `g{k}` announces `g{k} {version}` at
/// `start` and has one leg running `run-{k}`.
fn catalog(entries: usize, version: &str) -> Value {
    let groups: Vec<Value> = (0..entries)
        .map(|index| json!({
            "name": format!("g{index}"),
            "start": [{ "info": format!("g{index} {version}") }],
            "tasks": [{ "name": "leg", "shell": format!("run-{index}") }],
        }))
        .collect();
    json!({ "kind": "group-catalog", "groups": groups })
}

/// A one-step sequence whose step holds `steps` nested sequences; nested
/// sequence `s{k}` has one group step naming `g{k}@catalog.yaml`.
fn catalog_source(dir: &Path, steps: usize) -> String {
    write_yaml(dir, "catalog.yaml", &catalog(steps, "v1"));
    let children: Vec<Value> = (0..steps)
        .map(|index| json!({
            "name": format!("s{index}"),
            "sequence": [{ "name": "g", "group": format!("g{index}@catalog.yaml") }],
        }))
        .collect();
    one_step_source(dir, json!({ "name": "outer", "sequence": children }))
}

/// Runs every child command successfully; when it runs `trigger`, writes
/// `replacement` over `path`, or removes `path` when there is none.
struct RewritingShell {
    trigger: &'static str,
    path: PathBuf,
    replacement: Option<String>,
}

impl TaskShellRunner for RewritingShell {
    fn run(&self, command: &str, _timeout: Duration, _interrupt: Option<&AtomicBool>,
        _live: Option<&Arc<TaskLiveOutput>>) -> Result<ShellCommandOutput, TaskShellError>
    {
        if command == self.trigger {
            match &self.replacement {
                Some(text) => fs::write(&self.path, text).unwrap(),
                None => fs::remove_file(&self.path).unwrap(),
            }
        }
        Ok(ShellCommandOutput { stdout: command.into(), exit_code: 0, timed_out: false, interrupted: false, runaway: None })
    }
}

#[cfg(feature = "work-counters")]
fn run_commands(steps: usize) -> Vec<String> {
    (0..steps).map(|index| format!("run-{index}")).collect()
}

#[cfg(feature = "work-counters")]
#[test]
fn catalog_steps_share_one_parse_and_copy_no_entry_at_10_and_100() {
    for steps in [10, 100] {
        let dir = TempDir::new().unwrap();
        let catalog_nodes = nodes(&catalog(steps, "v1"));
        let before = snapshot();
        let source = catalog_source(dir.path(), steps);
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        let approved: Vec<String> = fixture.graph.shell_commands.iter().map(|command| command.command.clone()).collect();
        assert_eq!(approved, run_commands(steps), "every entry's leg joined the approval set, in order");
        assert_eq!(shell.commands(), run_commands(steps), "each step ran its own entry's leg");
        let expected: Vec<String> = (0..steps).map(|index| format!("info:g{index} v1")).collect();
        assert_eq!(recorder.events(), expected, "each step selected its own entry");
        assert_eq!(work.get(Work::ReferencedDocumentParse), 1, "{steps} references parsed the catalog once");
        assert_eq!(
            work.get(Work::ReferencedDocumentNodes), 2 * catalog_nodes,
            "one parse and one retired-read scan of the catalog; no reference copied it",
        );
    }
}

/// The invocation's root walk over `steps` group steps, step `s{k}` naming
/// `g{k}@catalog.yaml`, without the test fixture's seeded documents.
#[cfg(feature = "work-counters")]
#[test]
fn a_root_walk_over_catalog_steps_parses_the_catalog_once_at_10_and_100() {
    for steps in [10, 100] {
        let dir = TempDir::new().unwrap();
        write_yaml(dir.path(), "catalog.yaml", &catalog(steps, "v1"));
        let children: Vec<Value> = (0..steps)
            .map(|index| json!({ "name": format!("s{index}"), "group": format!("g{index}@catalog.yaml") }))
            .collect();
        let source = write_source(dir.path(), "seq.md", &[("sequence", Value::Array(children))], "Document body.\n");
        let resolved = crate::composition::resolve_fixture_source(&source).unwrap();
        let resolution = crate::test_support::context_for(&resolved.resolved_path);
        let plan = resolve_sequence_plan(&resolved, &resolution).unwrap().expect("fixture declares a sequence");

        let before = snapshot();
        let graph = crate::composition::sequence::preflight::build_preflight_graph(&plan, &resolved, &resolution).unwrap();
        let work = snapshot().since(&before);

        let approved: Vec<String> = graph.shell_commands.iter().map(|command| command.command.clone()).collect();
        assert_eq!(approved, run_commands(steps), "every entry's leg joined the approval set, in order");
        let selected: Vec<String> = graph.steps.iter().map(|step| match &step.task.as_ref().unwrap().action {
            PreflightAction::Group(group) => group.name.clone(),
            other => panic!("expected a group, got {other:?}"),
        }).collect();
        assert_eq!(selected, (0..steps).map(|index| format!("g{index}")).collect::<Vec<_>>());
        assert_eq!(work.get(Work::ReferencedDocumentParse), 1, "{steps} references parsed the catalog once");
        assert_eq!(work.get(Work::ReferencedDocumentNodes), 2 * nodes(&catalog(steps, "v1")), "no reference copied the catalog");
    }
}

#[cfg(feature = "work-counters")]
#[test]
fn referenced_sequence_offsets_share_one_parse_at_10_and_100() {
    for steps in [10, 100] {
        let dir = TempDir::new().unwrap();
        let lists: Map<String, Value> = (0..steps)
            .map(|index| (format!("s{index}"), json!([{ "name": "work", "shell": format!("run-{index}") }])))
            .collect();
        let lists = Value::Object(lists);
        write_yaml(dir.path(), "lists.yaml", &lists);
        let children: Vec<Value> = (0..steps)
            .map(|index| json!({ "name": format!("s{index}"), "sequence": format!("lists.yaml -> s{index}") }))
            .collect();
        let before = snapshot();
        let source = one_step_source(dir.path(), json!({ "name": "outer", "sequence": children }));
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands(), run_commands(steps), "each step ran its own offset's list");
        assert_eq!(work.get(Work::ReferencedDocumentParse), 1, "{steps} offsets parsed the file once");
        assert_eq!(work.get(Work::ReferencedDocumentNodes), 2 * nodes(&lists), "one parse and one scan; offsets copy nothing whole");
    }
}

#[test]
fn a_catalog_rewritten_between_steps_is_seen_by_the_next_preparation() {
    let dir = TempDir::new().unwrap();
    let source = catalog_source(dir.path(), 3);
    let path = dir.path().join("catalog.yaml");
    let replacement = serde_json::to_string_pretty(&catalog(3, "v2")).unwrap();
    let fixture = Fixture::build(dir, &source).unwrap();
    let shell = RewritingShell { trigger: "run-0", path, replacement: Some(replacement) };
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell;

    #[cfg(feature = "work-counters")]
    let before = snapshot();
    let outcome = fixture.execute(&wiring);

    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(recorder.events(), vec!["info:g0 v1", "info:g1 v2", "info:g2 v2"]);
    // The test fixture's root walk reads its own copies, so the first nested
    // preparation parses v1; the rewrite is parsed once for both later steps.
    #[cfg(feature = "work-counters")]
    assert_eq!(snapshot().since(&before).get(Work::ReferencedDocumentParse), 2, "v1 once, then v2 once");
}

#[test]
fn a_catalog_removed_between_steps_fails_the_next_preparation_as_a_fresh_preflight_would() {
    let dir = TempDir::new().unwrap();
    let source = catalog_source(dir.path(), 2);
    let path = dir.path().join("catalog.yaml");
    let fixture = Fixture::build(dir, &source).unwrap();
    let shell = RewritingShell { trigger: "run-0", path, replacement: None };
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell;

    let outcome = fixture.execute(&wiring);

    assert_eq!(outcome.status, TaskStatus::Failed);
    assert_eq!(recorder.events(), vec!["info:g0 v1"], "the second group never started");
    let Err(fresh) = Fixture::build(TempDir::new().unwrap(), &source) else {
        panic!("a preflight without the catalog must fail");
    };
    let expected = TaskDiagnostic::from_composition(TaskStage::Primary, &fresh).message().to_string();
    assert!(failure_message(&outcome).contains(&expected), "{}\nexpected: {expected}", failure_message(&outcome));
}
