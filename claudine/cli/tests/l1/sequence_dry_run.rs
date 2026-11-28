//! `claudine sequence --dry-run` has no effects: `shell:` and `side_effect:`
//! tasks are reported as not checked and never run, a loop is walked once,
//! and no lifecycle event or `prep` target runs. The report format itself is
//! covered by `dry_run_check.rs`.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run(fixture: &CliProcessFixture, flags: &[&str]) -> Run {
    let output = fixture
        .command()
        .args(["sequence", "--goose", "--yolo"])
        .args(flags)
        .arg(fixture.cwd().join("seq.md"))
        .output()
        .unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        status: output.status,
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

fn prose(text: &str) -> String {
    text.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A scratch repository with one untracked file and a sequence that stages
/// it. A fresh repository has no index file until something is staged.
#[cfg(unix)]
fn staging_fixture() -> CliProcessFixture {
    let fixture = CliProcessFixture::named("sequence-dry-run-git");
    assert!(crate::common::init_git_repo(fixture.cwd()), "git init");
    fs::write(fixture.cwd().join("change.txt"), "changed\n").unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: stage\n    shell: \"git add change.txt\"\n---\n").unwrap();
    assert!(!fixture.cwd().join(".git/index").exists(), "nothing is staged yet");
    fixture
}

/// Whether the scratch repository's index records `change.txt`, read from the
/// index file itself.
#[cfg(unix)]
fn staged(fixture: &CliProcessFixture) -> bool {
    fs::read(fixture.cwd().join(".git/index"))
        .is_ok_and(|index| index.windows(b"change.txt".len()).any(|window| window == b"change.txt"))
}

/// `--dry-run` reports `git add` as an unchecked command and leaves the
/// index untouched.
#[cfg(unix)]
#[test]
fn a_dry_run_reports_a_shell_task_and_leaves_the_index_untouched() {
    let fixture = staging_fixture();
    let run = run(&fixture, &["--dry-run"]);
    assert!(run.status.success(), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("not checked: command (a dry run runs no commands)"), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("Previewed 0 of 1 task."), "stderr:\n{}", run.stderr);
    assert!(!staged(&fixture), "the index is untouched");
}

/// `--dry-run-shell` was removed: alone it is a usage error that runs
/// nothing.
#[test]
fn dry_run_shell_without_dry_run_is_a_usage_error() {
    let fixture = CliProcessFixture::named("sequence-dry-run-usage");
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: a\n    side_effect: { append_line: [trace.log, a] }\n---\n").unwrap();
    let run = run(&fixture, &["--dry-run-shell"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("--dry-run"), "stderr:\n{}", run.stderr);
    assert!(!fixture.cwd().join("trace.log").exists());
}

/// A loop whose condition waits for a file an agent would write is walked
/// once and the check ends; its lifecycle events and its `prep` target never
/// run.
#[test]
fn a_dry_run_walks_a_file_dependent_loop_once_with_no_lifecycle_or_prep() {
    let fixture = CliProcessFixture::named("sequence-dry-run-loop");
    fs::write(
        fixture.cwd().join("seq.md"),
        r#"---
loop:
  until: "file_exists('agent-wrote.txt')"
  max: 50
initialize:
  - append_line: [trace.log, "initialize"]
  - prep: "@helper.md"
start:
  - append_line: [trace.log, "start"]
finalize:
  - append_line: [trace.log, "finalize"]
sequence:
  - name: work
    when: "loop.count > 1"
    side_effect: { append_line: [work.log, "pass {{ loop.count }}"] }
---
"#,
    )
    .unwrap();
    fs::write(fixture.cwd().join("helper.md"), "---\ninitialize:\n  - append_line: [helper.log, ran]\n---\nHelper\n").unwrap();
    let run = run(&fixture, &["--dry-run"]);
    assert!(run.status.success(), "stderr:\n{}", run.stderr);
    let prose = prose(&run.stderr);
    assert!(prose.contains("task work (sequence[0])"), "stderr:\n{}", run.stderr);
    assert!(prose.contains("Dry run found 0 errors and 0 warnings; 1 check not run."), "stderr:\n{}", run.stderr);
    for absent in ["trace.log", "helper.log", "work.log"] {
        assert!(!fixture.cwd().join(absent).exists(), "{absent} must not be written; stderr:\n{}", run.stderr);
    }
}

const SIDE_EFFECTS: &str = r#"---
flag: original
sequence:
  - name: write
    side_effect: { append_line: [scratch.txt, "{{ flag }}"] }
  - name: mark
    side_effect: { set: { flag: changed } }
  - name: read
    shell: "echo flag={{ flag }}"
---
"#;

fn side_effects_run(flags: &[&str]) -> (CliProcessFixture, Run) {
    let fixture = CliProcessFixture::named("sequence-dry-run-side-effects");
    fs::write(fixture.cwd().join("seq.md"), SIDE_EFFECTS).unwrap();
    let run = run(&fixture, flags);
    assert!(run.status.success(), "{flags:?}; stderr:\n{}", run.stderr);
    assert_eq!(prose(&run.stderr).matches("not checked: side effect (a dry run dispatches no side effects)").count(), 2, "{flags:?}; stderr:\n{}", run.stderr);
    assert!(!fixture.cwd().join("scratch.txt").exists(), "{flags:?}");
    (fixture, run)
}

/// A `side_effect:` task that writes a file and one that sets a runtime value
/// are reported as not checked and not run under `--dry-run`: the file stays
/// absent, and the `shell:` task is not run either.
#[test]
fn side_effect_tasks_are_reported_and_never_run_in_a_dry_run() {
    let (_fixture, run) = side_effects_run(&["--dry-run"]);
    assert!(prose(&run.stderr).contains("not checked: command (a dry run runs no commands)"), "stderr:\n{}", run.stderr);
    assert!(!run.stdout.contains("flag="), "stdout:\n{}", run.stdout);
}

/// An inline container's `$schema` requiring a value that only a runtime
/// write supplies is not failed by a dry run, which does not run the write.
/// A real run then admits the container with the written value.
fn assert_runtime_write_satisfies_the_inline_check(writer: &str) {
    const CONTAINER: &str = "  - name: review\n    $schema: { reviewer: string(required) }\n    sequence: [{ name: sign, side_effect: { append_line: [signed.log, \"{{ reviewer }}\"] } }]\n";
    let fixture = CliProcessFixture::named("sequence-dry-run-inline-check");
    fs::write(fixture.cwd().join("seq.md"), format!("---\n{writer}{CONTAINER}---\n")).unwrap();

    let preview = run(&fixture, &["--dry-run"]);
    assert!(preview.status.success(), "{writer}\nstderr:\n{}", preview.stderr);
    assert!(prose(&preview.stderr).contains("Dry run found 0 errors and 0 warnings"), "{writer}\nstderr:\n{}", preview.stderr);
    assert!(!fixture.cwd().join("signed.log").exists(), "{writer}");

    let real = run(&fixture, &["--silent"]);
    assert!(real.status.success(), "{writer}\nstderr:\n{}", real.stderr);
    assert_eq!(fs::read_to_string(fixture.cwd().join("signed.log")).unwrap().trim(), "ada", "{writer}");
}

#[test]
fn a_dry_run_does_not_fail_an_inline_check_that_an_earlier_set_satisfies() {
    assert_runtime_write_satisfies_the_inline_check("sequence:\n  - { name: mark, side_effect: { set: { reviewer: ada } } }\n");
}

#[test]
fn a_dry_run_does_not_fail_an_inline_check_that_initialize_satisfies() {
    assert_runtime_write_satisfies_the_inline_check("initialize: [{ set: { reviewer: ada } }]\nsequence:\n");
}
