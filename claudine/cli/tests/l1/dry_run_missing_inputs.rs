//! A missing input under `--dry-run` makes unavailable only what reads it.
//!
//! The input is one `composition.missing_properties` finding with its
//! remedy; what reads it stays in the preview as written and marked
//! `[missing input]`; and every check that does not read it still runs, so
//! an independent broken `::file` is reported beside it and the static text
//! around both is previewed. One test per route: direct compose, inline
//! compose, a sequence composing its own body, a sequence prompt task, a
//! referenced sequence document's body (a prompt reading a missing
//! sequence-owner input is `dry_run_check`'s nested-task test). Each
//! also runs its control, with the input supplied and the reference
//! repaired, which checks clean.

use std::fs;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    let output = fixture.command().env("NO_COLOR", "1").args(args).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        code: output.status.code(),
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// `text` with each run of whitespace as one space, so a wrapped row reads as
/// one line.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A fixture with a `claude` on its `PATH` that the check only looks up.
fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let path = fixture.bin_dir().join(format!("claude{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fixture
}

/// The codes of a `--json` report's findings, in order.
fn codes(run: &Run) -> Vec<String> {
    let report: Value = serde_json::from_str(&run.stdout).unwrap_or_else(|_| panic!("not JSON:\n{}", run.stdout));
    report["diagnostics"].as_array().unwrap().iter().map(|row| row["code"].as_str().unwrap().to_string()).collect()
}

const MISSING: &str = "composition.missing_properties";
const MARKED: &str = "{{ topic }} [missing input]";

/// The rows both findings print: the input with its remedy, and the
/// independent inclusion.
fn assert_both_findings(run: &Run, remedy: &str) {
    let report = prose(&run.stderr);
    assert_eq!(run.code, Some(2), "stderr:\n{}", run.stderr);
    assert_eq!(report.matches("`topic` (string) is required and has no value").count(), 1, "{report}");
    assert!(report.contains(remedy), "{report}");
    assert!(report.contains("file not found in `::file` ::file ./gone.md"), "{report}");
}

/// Direct compose: a required `topic` with no value and a broken `::file`
/// beside it are both reported, and the static text is previewed with the
/// placeholder marked. Supplied and repaired, it checks clean.
#[test]
fn compose_reports_the_missing_input_and_the_independent_inclusion_and_previews_the_rest() {
    let fixture = fixture("dry-run-missing-compose");
    let doc = "---\nagent: claude\n$schema:\n  topic: string(required)\n---\nStatic intro.\n\nTopic is {{ topic }}.\n\n::file ./gone.md\n";
    fs::write(fixture.cwd().join("doc.md"), doc).unwrap();

    let missing = run(&fixture, &["compose", "doc.md", "--dry-run"]);
    assert_both_findings(&missing, "hint: pass `topic=<value>` after the file, or give it a default");
    assert!(missing.stdout.contains("Static intro."), "stdout:\n{}", missing.stdout);
    assert!(missing.stdout.contains(&format!("Topic is {MARKED}.")), "stdout:\n{}", missing.stdout);

    fs::write(fixture.cwd().join("gone.md"), "Included.\n").unwrap();
    let control = run(&fixture, &["compose", "doc.md", "topic=docs", "--dry-run"]);
    assert_eq!(control.code, Some(0), "stderr:\n{}", control.stderr);
    assert_eq!(control.stdout.trim(), "Static intro.\n\nTopic is docs.\n\nIncluded.", "stderr:\n{}", control.stderr);
}

/// Supplying the input removes only the input's finding; repairing the
/// inclusion removes only the inclusion's.
#[test]
fn each_repair_removes_only_its_own_finding() {
    let fixture = fixture("dry-run-missing-repairs");
    let doc = "---\nagent: claude\n$schema:\n  topic: string(required)\n---\nTopic is {{ topic }}.\n\n::file ./gone.md\n";
    fs::write(fixture.cwd().join("doc.md"), doc).unwrap();

    let supplied = run(&fixture, &["compose", "doc.md", "topic=docs", "--dry-run", "--json"]);
    assert_eq!(codes(&supplied), ["dm.reference.missing_transclusion"], "{}", supplied.stdout);

    fs::write(fixture.cwd().join("gone.md"), "Included.\n").unwrap();
    let repaired = run(&fixture, &["compose", "doc.md", "--dry-run", "--json"]);
    assert_eq!(codes(&repaired), [MISSING], "{}", repaired.stdout);
}

/// Inline compose: a missing `string(required; eager)` the frontmatter
/// `prompt:` reads is reported and marked, while a completion-only required
/// value stays absent without a finding, as at the inline launch.
#[test]
fn inline_compose_reports_an_eager_input_and_lets_a_completion_only_one_stay_absent() {
    let fixture = fixture("dry-run-missing-inline");
    let doc = "---\nagent: claude\n$schema:\n  topic: \"string(required; eager)\"\n  summary: string(required)\nprompt: \"Static intro about {{ topic }}.\"\n---\n";
    fs::write(fixture.cwd().join("notes.md"), doc).unwrap();

    let missing = run(&fixture, &["inline-compose", "notes.md", "--dry-run"]);
    let report = prose(&missing.stderr);
    assert_eq!(missing.code, Some(2), "stderr:\n{}", missing.stderr);
    assert_eq!(report.matches("has no value").count(), 1, "only `topic` is reported:\n{report}");
    assert!(report.contains("`topic` (string) is required and has no value"), "{report}");
    assert!(missing.stdout.contains(&format!("Static intro about {MARKED}.")), "stdout:\n{}", missing.stdout);

    let control = run(&fixture, &["inline-compose", "notes.md", "topic=docs", "--dry-run"]);
    assert_eq!(control.code, Some(0), "stderr:\n{}", control.stderr);
    assert!(control.stdout.contains("Static intro about docs."), "stdout:\n{}", control.stdout);
}

/// A sequence step composing its document's body: the root owner's missing
/// input is reported once, and the body is previewed beside the broken
/// inclusion.
#[test]
fn a_sequence_body_is_previewed_beside_its_missing_input() {
    let fixture = fixture("dry-run-missing-sequence-body");
    let doc = "---\nagent: claude\n$schema:\n  topic: string(required)\nsequence:\n  - name: intro\n---\nStatic root intro about {{ topic }}.\n\n::file ./gone.md\n";
    fs::write(fixture.cwd().join("seq.md"), doc).unwrap();

    let missing = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    assert_both_findings(&missing, "hint: pass `topic=<value>` after the file, or give it a default");
    assert!(missing.stdout.contains(&format!("Static root intro about {MARKED}.")), "stdout:\n{}", missing.stdout);

    fs::write(fixture.cwd().join("gone.md"), "Included.\n").unwrap();
    let control = run(&fixture, &["sequence", "seq.md", "topic=docs", "--dry-run"]);
    assert_eq!(control.code, Some(0), "stderr:\n{}", control.stderr);
    assert_eq!(control.stdout.trim(), "Static root intro about docs.\n\nIncluded.", "stderr:\n{}", control.stderr);
}

/// A sequence prompt task missing its own input: the input finding, the
/// task's independent reference finding, and the rest of its preview.
#[test]
fn a_prompt_task_keeps_its_reference_check_and_preview_beside_its_missing_input() {
    let fixture = fixture("dry-run-missing-prompt-task");
    fs::write(fixture.cwd().join("seq.md"), "---\nagent: claude\nsequence:\n  - name: write\n    prompt: ./task.md\n---\n").unwrap();
    let task = "---\n$schema:\n  topic: string(required)\n---\nTask intro on {{ topic }}.\n\n::file ./gone.md\n";
    fs::write(fixture.cwd().join("task.md"), task).unwrap();

    let missing = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    assert_both_findings(&missing, "hint: add `topic` to the task's `with:`, or pass `topic=<value>` after the file");
    assert!(missing.stdout.contains(&format!("Task intro on {MARKED}.")), "stdout:\n{}", missing.stdout);

    fs::write(fixture.cwd().join("gone.md"), "Included.\n").unwrap();
    let control = run(&fixture, &["sequence", "seq.md", "topic=docs", "--dry-run"]);
    assert_eq!(control.code, Some(0), "stderr:\n{}", control.stderr);
    assert_eq!(control.stdout.trim(), "Task intro on docs.\n\nIncluded.", "stderr:\n{}", control.stderr);
}

/// A referenced sequence document's body step: its owner's missing input is
/// reported once, under the referencing task, and the body is previewed.
#[test]
fn a_referenced_sequence_body_reports_its_input_once_and_is_previewed() {
    let fixture = fixture("dry-run-missing-referenced-body");
    fs::write(fixture.cwd().join("outer.md"), "---\nagent: claude\nsequence:\n  - name: child\n    sequence: ./child.md\n---\n").unwrap();
    let child = "---\nagent: claude\n$schema:\n  topic: string(required)\nsequence:\n  - name: body\n---\nChild body about {{ topic }}.\n";
    fs::write(fixture.cwd().join("child.md"), child).unwrap();

    let missing = run(&fixture, &["sequence", "outer.md", "--dry-run"]);
    let report = prose(&missing.stderr);
    assert_eq!(missing.code, Some(2), "stderr:\n{}", missing.stderr);
    assert_eq!(report.matches("`topic` (string) is required and has no value").count(), 1, "{report}");
    assert!(missing.stdout.contains(&format!("Child body about {MARKED}.")), "stdout:\n{}", missing.stdout);

    let control = run(&fixture, &["sequence", "outer.md", "topic=docs", "--dry-run"]);
    assert_eq!(control.code, Some(0), "stderr:\n{}", control.stderr);
    assert_eq!(control.stdout.trim(), "Child body about docs.", "stderr:\n{}", control.stderr);
}
