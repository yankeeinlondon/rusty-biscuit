//! Nested preparation rereads a container's authoring document at every
//! container start but parses it only when its text changed: sibling
//! containers authored in one unchanged file share one parse, and a file a
//! task rewrote is parsed again and seen exactly as a fresh read sees it.

use super::*;
use crate::work_counters::{Work, snapshot};

/// A one-step sequence whose step holds `containers` sibling inline
/// sequences. Each announces `{name} {version}` at `start` and runs one
/// shell child, `run-{index}`.
fn source_with(dir: &Path, containers: usize, version: &str) -> String {
    let children: Vec<Value> = (0..containers)
        .map(|index| json!({
            "name": format!("c{index}"),
            "start": [{ "info": format!("c{index} {version}") }],
            "sequence": [{ "name": "work", "shell": format!("run-{index}") }],
        }))
        .collect();
    one_step_source(dir, json!({ "name": "outer", "sequence": children }))
}

/// Runs every child command successfully and, when it runs `trigger`,
/// replaces the source file with `replacement` before returning.
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

#[test]
fn sibling_containers_in_one_unchanged_file_share_one_parse_at_10_and_100() {
    for containers in [10, 100] {
        let dir = TempDir::new().unwrap();
        let source = source_with(dir.path(), containers, "v1");
        let fixture = Fixture::build(dir, &source).unwrap();
        let recorder = Recorder::default();
        let shell = FakeTaskShell::default();

        let before = snapshot();
        let outcome = fixture.execute(&Wiring::new(&recorder, &shell));
        let work = snapshot().since(&before);

        assert!(outcome.succeeded(), "{}", failure_message(&outcome));
        assert_eq!(shell.commands().len(), containers);
        assert_eq!(recorder.events().len(), containers, "every container prepared and started");
        assert_eq!(work.get(Work::NestedSourceParse), 1, "{containers} containers parsed their unchanged source once");
    }
}

#[test]
fn a_container_after_a_rewrite_sees_the_new_events_and_its_sibling_reuses_them() {
    let dir = TempDir::new().unwrap();
    let source = source_with(dir.path(), 3, "v1");
    let fixture = Fixture::build(dir, &source).unwrap();
    let replacement = fs::read_to_string(source_with(fixture._dir.path(), 3, "v2")).unwrap();
    // `Fixture::build` already audited v1; put it back for the run.
    source_with(fixture._dir.path(), 3, "v1");
    let shell = RewritingShell { trigger: "run-0", path: PathBuf::from(&source), replacement };
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell;

    let before = snapshot();
    let outcome = fixture.execute(&wiring);
    let work = snapshot().since(&before);

    assert!(outcome.succeeded(), "{}", failure_message(&outcome));
    assert_eq!(recorder.events(), vec!["info:c0 v1", "info:c1 v2", "info:c2 v2"]);
    assert_eq!(work.get(Work::NestedSourceParse), 2, "v1 once, then v2 once for both later containers");
}

#[test]
fn a_rewrite_that_breaks_the_source_fails_the_next_container_with_the_fresh_parse_error() {
    let dir = TempDir::new().unwrap();
    let source = source_with(dir.path(), 2, "v1");
    let fixture = Fixture::build(dir, &source).unwrap();
    let broken = "---\nsequence: [unterminated\n---\n\nDocument body.\n".to_string();
    let shell = RewritingShell { trigger: "run-0", path: PathBuf::from(&source), replacement: broken };
    let recorder = Recorder::default();
    let fake = FakeTaskShell::default();
    let mut wiring = Wiring::new(&recorder, &fake);
    wiring.shell = &shell;

    let outcome = fixture.execute(&wiring);

    assert_eq!(outcome.status, TaskStatus::Failed);
    assert_eq!(recorder.events(), vec!["info:c0 v1"], "the second container never started");
    let path = &fixture.graph.steps[0].task.as_ref().unwrap().origin_path;
    let fresh = CompositionError::MarkdownLoad {
        path: path.clone(),
        source: crate::composition::error::MarkdownLoadCause::Parse(Box::new(
            darkmatter::markdown::Markdown::try_from(path.as_path()).unwrap_err(),
        )),
    };
    let expected = TaskDiagnostic::from_composition(TaskStage::Primary, &fresh).message().to_string();
    assert!(failure_message(&outcome).contains(&expected), "{}\nexpected: {expected}", failure_message(&outcome));
}
