//! A schema trigger discovery skips as invalid is a warning in Claudine's
//! ordinary launch, never a fatal `CompositionError: schema load failed`
//! blamed on the prompt.
//!
//! The incident: a prompt with no `$schema` beside an unfinished
//! `schemas/memory.yaml` (unsupported `name`/`description` keys, null
//! `match:`) failed before any provider launched. Each test places that
//! trigger in the fixture home's `~/schemas` root and runs the real
//! `claudine` binary with the compiled `claudine-fake-goose`, which records
//! each launch, so the tests run on every OS and never reach a real provider.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi};

/// The trigger from the original incident.
const MEMORY_TRIGGER: &str = "kind: schema-trigger\nname: Memory Trigger\ndescription: TODO\nmatch:\n";

/// The start of every row Darkmatter words for a skipped trigger.
const WARNING: &str = "ignoring invalid schema trigger";

struct Run {
    code: Option<i32>,
    /// ANSI-free stderr with whitespace and block gutters collapsed, so
    /// substring checks survive terminal-width wrapping.
    stderr: String,
    launches: usize,
}

impl Run {
    fn warnings(&self) -> usize {
        self.stderr.matches(WARNING).count()
    }
}

/// A fixture with a launch-recording `goose` and the incident trigger in
/// `~/schemas/memory.yaml`; returns the trigger's path.
fn incident_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let trigger = fixture.home().join("schemas").join("memory.yaml");
    fs::create_dir_all(trigger.parent().unwrap()).unwrap();
    fs::write(&trigger, MEMORY_TRIGGER).unwrap();
    (fixture, trigger)
}

fn write_doc(fixture: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(name);
    fs::write(&path, content).unwrap();
    path
}

fn run(fixture: &CliProcessFixture, subcommand: &str, doc: &Path) -> Run {
    let marker = fixture.cwd().join("provider-launched.txt");
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        // An inline run expects its agent to write the body.
        .env("FAKE_INLINE_DOCUMENT", doc)
        .args([subcommand, "--goose", doc.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr))
        .replace(['┃', '│'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Run {
        code: output.status.code(),
        stderr,
        launches: fs::read_to_string(&marker).map_or(0, |log| log.lines().count()),
    }
}

/// The warning names the trigger file and why it was skipped, and nothing
/// renders it as a schema-load failure of the prompt.
fn assert_incident_warning(run: &Run, trigger: &Path, prompt: &str) {
    assert_eq!(run.warnings(), 1, "the trigger is reported exactly once:\n{}", run.stderr);
    let file_name = trigger.file_name().unwrap().to_str().unwrap();
    assert!(run.stderr.contains(file_name), "names the trigger file:\n{}", run.stderr);
    assert!(run.stderr.contains("found unknown keys: name, description"), "names the reason:\n{}", run.stderr);
    assert!(!run.stderr.contains("schema load failed"), "never a schema-load error:\n{}", run.stderr);
    let warning_row = &run.stderr[run.stderr.find(WARNING).unwrap()..];
    let warning_row = &warning_row[..warning_row.find(" found unknown keys").unwrap()];
    assert!(!warning_row.contains(prompt), "never attributed to the prompt:\n{}", run.stderr);
}

#[test]
fn compose_launches_beside_the_incident_trigger_and_warns_once() {
    let (fixture, trigger) = incident_fixture("trigger-warn-compose");
    let doc = write_doc(&fixture, "merge-conflicts.md", "---\ntitle: Merge\n---\nResolve the {{ title }} conflicts.\n");

    let run = run(&fixture, "compose", &doc);

    assert_eq!(run.code, Some(0), "the prompt launches:\n{}", run.stderr);
    assert_eq!(run.launches, 1, "the provider ran once:\n{}", run.stderr);
    assert!(!run.stderr.contains("CompositionError"), "no composition error:\n{}", run.stderr);
    assert_incident_warning(&run, &trigger, "merge-conflicts.md");
}

#[test]
fn inline_compose_launches_beside_the_incident_trigger_and_warns_once() {
    let (fixture, trigger) = incident_fixture("trigger-warn-inline");
    let doc = write_doc(&fixture, "inline.md", "---\nprompt: Summarize the repository\n---\n");

    let run = run(&fixture, "inline-compose", &doc);

    assert_eq!(run.code, Some(0), "the prompt launches:\n{}", run.stderr);
    assert_eq!(run.launches, 1, "the provider ran once:\n{}", run.stderr);
    assert_incident_warning(&run, &trigger, "inline.md");
}

/// Every loop iteration prepares the document again and rediscovers the
/// trigger; the invocation still shows its warning once.
#[test]
fn every_loop_iteration_rediscovers_the_trigger_and_it_is_shown_once() {
    let (fixture, trigger) = incident_fixture("trigger-warn-loop");
    let doc = write_doc(
        &fixture,
        "loop.md",
        "---\nloop:\n  while: \"counter < 2\"\n  actions:\n    - \"increment(counter)\"\n---\nCount is {{counter}}\n",
    );

    let run = run(&fixture, "compose", &doc);

    assert_eq!(run.code, Some(0), "the loop completes:\n{}", run.stderr);
    assert_eq!(run.launches, 3, "three iterations launched:\n{}", run.stderr);
    assert_incident_warning(&run, &trigger, "loop.md");
}

/// A valid trigger beside the invalid one still enforces its schema: the
/// prompt is rejected before any launch, and the skipped trigger's warning
/// is delivered with that rejection, ahead of it.
#[test]
fn a_rejected_prompt_shows_the_skipped_trigger_before_its_error() {
    let (fixture, trigger) = incident_fixture("trigger-warn-rejected");
    fs::write(
        fixture.home().join("schemas").join("a-gate.yaml"),
        "kind: schema-trigger\nmatch:\n  gate: string(required)\n$schema:\n  count: number(required)\n",
    )
    .unwrap();
    let doc = write_doc(&fixture, "gated.md", "---\ngate: yes\ncount: many\n---\nbody\n");

    let run = run(&fixture, "compose", &doc);

    assert_eq!(run.code, Some(1), "the valid trigger rejects `count`:\n{}", run.stderr);
    assert_eq!(run.launches, 0, "nothing launched:\n{}", run.stderr);
    assert!(run.stderr.contains("/count"), "the schema verdict names `count`:\n{}", run.stderr);
    assert_incident_warning(&run, &trigger, "gated.md");
    assert!(
        run.stderr.find(WARNING) < run.stderr.find("/count"),
        "the warning precedes the error:\n{}",
        run.stderr
    );
}

/// An authored `$schema` that cannot load is still a fatal schema-load error
/// about that reference, and the skipped trigger is still reported with it.
#[test]
fn an_authored_schema_that_cannot_load_still_fails_and_reports_the_trigger() {
    let (fixture, trigger) = incident_fixture("trigger-warn-missing-schema");
    let doc = write_doc(&fixture, "authored.md", "---\n$schema: ./missing.yaml\n---\nbody\n");

    let run = run(&fixture, "compose", &doc);

    assert_ne!(run.code, Some(0), "the missing authored schema fails:\n{}", run.stderr);
    assert_eq!(run.launches, 0, "nothing launched:\n{}", run.stderr);
    assert!(run.stderr.contains("missing.yaml"), "the error names the authored reference:\n{}", run.stderr);
    assert_eq!(run.warnings(), 1, "the trigger is reported exactly once:\n{}", run.stderr);
    assert!(run.stderr.contains(trigger.file_name().unwrap().to_str().unwrap()), "{}", run.stderr);
}

/// An authored `$schema` naming the skipped trigger file never reads it as a
/// schema: the launch still fails.
#[test]
fn an_authored_schema_naming_the_skipped_trigger_still_fails() {
    let (fixture, trigger) = incident_fixture("trigger-warn-authored-trigger");
    let doc = write_doc(
        &fixture,
        "names-trigger.md",
        &format!("---\n$schema: {}\n---\nbody\n", biscuit_file::to_portable_string(&trigger)),
    );

    let run = run(&fixture, "compose", &doc);

    assert_ne!(run.code, Some(0), "a trigger file is not a document schema:\n{}", run.stderr);
    assert_eq!(run.launches, 0, "nothing launched:\n{}", run.stderr);
}
